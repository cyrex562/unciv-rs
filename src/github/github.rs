use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::thread;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use reqwest::{Client, header, StatusCode};
use crate::json::{from_json_file, to_json};
use crate::models::ModOptions;
use crate::utils::Log;

/// Utility managing Github access
///
/// Singleton - RateLimit is shared app-wide and has local variables, and is not tested for thread safety.
/// Therefore, additional effort is required should `try_get_github_repos_with_topic` ever be called non-sequentially.
/// `download` and `download_and_extract` should be thread-safe as they are self-contained.
/// They do not join in the `RateLimit` handling because Github doc suggests each API
/// has a separate limit (and I found none for cloning via a zip).
pub struct Github;

impl Github {
    const CONTENT_DISPOSITION_HEADER: &'static str = "Content-Disposition";
    const ATTACHMENT_DISPOSITION_PREFIX: &'static str = "attachment;filename=";
    const OUTER_BLANK_REPLACEMENT: char = '=';

    /// Helper opens a url and accesses its input stream, logging errors to the console
    ///
    /// # Arguments
    ///
    /// * `url` - String representing a URL to download.
    /// * `pre_download_action` - Optional callback that will be executed between opening the connection and
    ///   accessing its data - passes the connection and allows e.g. reading the response headers.
    ///
    /// # Returns
    ///
    /// The response body as a Vec<u8> if successful, `None` otherwise.
    pub fn download<F>(url: &str, pre_download_action: Option<F>) -> Option<Vec<u8>>
    where
        F: FnOnce(&reqwest::Response) -> ()
    {
        let client = Client::new();

        match client.get(url).send() {
            Ok(mut response) => {
                // Execute pre-download action if provided
                if let Some(action) = pre_download_action {
                    action(&response);
                }

                // Get the response body
                match response.bytes() {
                    Ok(bytes) => Some(bytes.to_vec()),
                    Err(e) => {
                        // Log error and return None
                        Log::error("Exception during GitHub download: {}", e);
                        None
                    }
                }
            },
            Err(e) => {
                Log::error("Exception during GitHub download: {}", e);
                None
            }
        }
    }

    /// Download a mod and extract, deleting any pre-existing version.
    ///
    /// # Arguments
    ///
    /// * `repo` - The GitHub repository information
    /// * `mods_folder` - Destination path of mods folder
    /// * `update_progress_percent` - Optional callback that accepts a number 0-100 for progress updates
    ///
    /// # Returns
    ///
    /// Path to the downloaded Mod's folder or None if download failed
    pub fn download_and_extract<F>(
        repo: &GithubAPI::Repo,
        mods_folder: &Path,
        update_progress_percent: Option<F>
    ) -> Option<PathBuf>
    where
        F: Fn(i32) + Send + 'static
    {
        let mut mod_name_from_file_name = repo.name.clone();

        let default_branch = &repo.default_branch;
        let zip_url: String;
        let temp_name: String;

        if repo.direct_zip_url.is_empty() {
            let git_repo_url = &repo.html_url;
            // Initiate download - the helper returns None when it fails
            zip_url = GithubAPI::get_url_for_branch_zip(git_repo_url, default_branch);

            // Get a mod-specific temp file name
            temp_name = format!("temp-{:x}", git_repo_url.hash());
        } else {
            zip_url = repo.direct_zip_url.clone();
            temp_name = format!("temp-{:x}", repo.to_string().hash());
        }

        let mut content_length = 0;
        let input_stream = Self::download(&zip_url, Some(|response| {
            // We DO NOT want to accept "Transfer-Encoding: chunked" here, as we need to know the size for progress tracking
            // So this attempts to limit the encoding to gzip only
            // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Transfer-Encoding
            // HOWEVER it doesn't seem to work - the server still sends chunked data sometimes
            // which means we don't actually know the total length :(
            let headers = response.headers();

            if let Some(disposition) = headers.get(Self::CONTENT_DISPOSITION_HEADER) {
                if let Ok(disposition_str) = disposition.to_str() {
                    if disposition_str.starts_with(Self::ATTACHMENT_DISPOSITION_PREFIX) {
                        mod_name_from_file_name = disposition_str
                            .trim_start_matches(Self::ATTACHMENT_DISPOSITION_PREFIX)
                            .trim_end_matches(".zip")
                            .replace('.', " ");
                    }
                }
            }

            if let Some(length) = headers.get("Content-Length") {
                if let Ok(length_str) = length.to_str() {
                    if let Ok(length_val) = length_str.parse::<i32>() {
                        content_length = length_val;
                    }
                }
            }
        }))?;

        // Download to temporary zip
        let temp_zip_path = mods_folder.join(format!("{}.zip", temp_name));
        let mut file = File::create(&temp_zip_path)?;
        file.write_all(&input_stream)?;

        // prepare temp unpacking folder
        let unzip_destination = mods_folder.join(&temp_name); // folder, not file
        // prevent mixing new content with old - hopefully there will never be cadavers of our tempZip stuff
        if unzip_destination.exists() {
            if unzip_destination.is_dir() {
                fs::remove_dir_all(&unzip_destination)?;
            } else {
                fs::remove_file(&unzip_destination)?;
            }
        }

        // Extract the zip file
        let zip_file = File::open(&temp_zip_path)?;
        let mut archive = zip::ZipArchive::new(zip_file)?;
        archive.extract(&unzip_destination)?;

        let (inner_folder, mod_name) = Self::resolve_zip_structure(&unzip_destination, &mod_name_from_file_name)?;

        // modName can be "$repoName-$defaultBranch"
        let final_destination_name = mod_name.replace(&format!("-{}", default_branch), "").repo_name_to_folder_name();
        // finalDestinationName is now the mod name as we display it. Folder name needs to be identical.
        let final_destination = mods_folder.join(&final_destination_name);

        // prevent mixing new content with old
        let mut temp_backup = None;
        if final_destination.exists() {
            temp_backup = Some(final_destination.with_file_name(format!("{}.updating", final_destination_name)));
            if final_destination.is_dir() {
                fs::rename(&final_destination, &temp_backup.as_ref().unwrap())?;
            } else {
                fs::rename(&final_destination, &temp_backup.as_ref().unwrap())?;
            }
        }

        // Move temp unpacked content to their final place
        fs::create_dir_all(&final_destination)?; // If we don't create this as a directory, it will think this is a file and nothing will work.
        // The move will reset the last modified time (recursively, at least on Linux)
        // This sort will guarantee the desktop launcher will not re-pack textures and overwrite the atlas as delivered by the mod
        let mut entries: Vec<_> = fs::read_dir(&inner_folder)?
            .filter_map(|e| e.ok())
            .collect();

        entries.sort_by(|a, b| {
            let a_is_atlas = a.path().extension().map_or(false, |ext| ext == "atlas");
            let b_is_atlas = b.path().extension().map_or(false, |ext| ext == "atlas");
            a_is_atlas.cmp(&b_is_atlas)
        });

        for entry in entries {
            let source = entry.path();
            let dest = final_destination.join(source.file_name().unwrap());
            if source.is_dir() {
                fs::rename(&source, &dest)?;
            } else {
                fs::rename(&source, &dest)?;
            }
        }

        // clean up
        fs::remove_file(&temp_zip_path)?;
        fs::remove_dir_all(&unzip_destination)?;
        if let Some(backup) = temp_backup {
            if backup.is_dir() {
                fs::remove_dir_all(&backup)?;
            } else {
                fs::remove_file(&backup)?;
            }
        }

        Some(final_destination)
    }

    /// Checks if a directory is a valid mod folder
    fn is_valid_mod_folder(dir: &Path) -> bool {
        let good_folders = vec![
            "Images", "jsons", "maps", "music", "sounds", "Images\\..*", "scenarios"
        ].iter()
        .map(|s| regex::Regex::new(s).unwrap())
        .collect::<Vec<_>>();

        let good_files = vec![
            ".*\\.atlas", ".*\\.png", "preview.jpg", ".*\\.md", "Atlases.json", ".nomedia", "license"
        ].iter()
        .map(|s| regex::Regex::new(s).unwrap())
        .collect::<Vec<_>>();

        let mut good = 0;
        let mut bad = 0;

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                let is_good = if path.is_dir() {
                    good_folders.iter().any(|re| re.is_match(name))
                } else {
                    good_files.iter().any(|re| re.is_match(name))
                };

                if is_good {
                    good += 1;
                } else {
                    bad += 1;
                }
            }
        }

        good > 0 && good > bad
    }

    /// Check whether the unpacked zip contains a subfolder with mod content or is already the mod.
    /// If there's a subfolder we'll assume it is the mod name, optionally suffixed with branch or release-tag name like github does.
    ///
    /// # Returns
    ///
    /// Tuple of (actual mod content folder path, mod name)
    fn resolve_zip_structure(dir: &Path, default_mod_name: &str) -> io::Result<(PathBuf, String)> {
        if Self::is_valid_mod_folder(dir) {
            return Ok((dir.to_path_buf(), default_mod_name.to_string()));
        }

        let subdirs: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        if subdirs.len() == 1 && Self::is_valid_mod_folder(&subdirs[0].path()) {
            let name = subdirs[0].path().file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            return Ok((subdirs[0].path(), name));
        }

        Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid Mod archive structure"))
    }

    /// Query GitHub for repositories marked "unciv-mod"
    ///
    /// # Arguments
    ///
    /// * `amount_per_page` - Number of search results to return for this request.
    /// * `page` - The "page" number, starting at 1.
    /// * `search_request` - Optional search query string.
    ///
    /// # Returns
    ///
    /// Parsed `RepoSearch` json on success, `None` on failure.
    pub fn try_get_github_repos_with_topic(amount_per_page: i32, page: i32, search_request: &str) -> Option<GithubAPI::RepoSearch> {
        let link = GithubAPI::get_url_for_mod_listing(search_request, amount_per_page, page);
        let mut retries = 2;

        while retries > 0 {
            retries -= 1;
            // obey rate limit
            if RateLimit::wait_for_limit() {
                return None;
            }

            // try download
            let response = Self::download(&link, Some(|response| {
                if response.status() == StatusCode::FORBIDDEN ||
                   (response.status() == StatusCode::OK && page == 1 && retries == 1) {
                    // Pass the response headers to the rate limit handler so it can process the rate limit headers
                    RateLimit::notify_http_response(response);
                    retries += 1; // An extra retry so the 403 is ignored in the retry count
                }
            }))?;

            let text = String::from_utf8_lossy(&response);
            match serde_json::from_str::<GithubAPI::RepoSearch>(&text) {
                Ok(result) => return Some(result),
                Err(e) => {
                    Log::error("Failed to parse Github response as json - {}: {}", text, e);
                    return None;
                }
            }
        }

        None
    }

    /// Get a Pixmap from a "preview" png or jpg file at the root of the repo, falling back to the
    /// repo owner's avatar `avatar_url`. The file content url is constructed from `mod_url` and `default_branch`
    /// by replacing the host with `raw.githubusercontent.com`.
    pub fn try_get_preview_image(mod_url: &str, default_branch: &str, avatar_url: Option<&str>) -> Option<Vec<u8>> {
        // Side note: github repos also have a "Social Preview" optionally assignable on the repo's
        // settings page, but that info is inaccessible using the v3 API anonymously. The easiest way
        // to get it would be to query the the repo's frontend page (modUrl), and parse out
        // `head/meta[property=og:image]/@content`, which is one extra spurious roundtrip and a
        // non-trivial waste of bandwidth.
        // Thus we ask for a "preview" file as part of the repo contents instead.
        let file_location = GithubAPI::get_url_for_preview(mod_url, default_branch);

        // Try jpg first, then png, then avatar
        Self::download(&format!("{}.jpg", file_location))
            .or_else(|| Self::download(&format!("{}.png", file_location)))
            .or_else(|| avatar_url.and_then(|url| Self::download(url)))
    }

    /// Queries github for a tree and calculates the sum of the blob sizes.
    ///
    /// # Returns
    ///
    /// -1 on failure, else size rounded to kB
    pub fn get_repo_size(repo: &GithubAPI::Repo) -> i32 {
        let link = repo.get_url_for_tree_query();
        let mut retries = 2;

        while retries > 0 {
            retries -= 1;
            // obey rate limit
            if RateLimit::wait_for_limit() {
                return -1;
            }

            // try download
            let response = Self::download(&link, Some(|response| {
                if response.status() == StatusCode::FORBIDDEN ||
                   (response.status() == StatusCode::OK && retries == 1) {
                    // Pass the response headers to the rate limit handler so it can process the rate limit headers
                    RateLimit::notify_http_response(response);
                    retries += 1; // An extra retry so the 403 is ignored in the retry count
                }
            }))?;

            let text = String::from_utf8_lossy(&response);
            match serde_json::from_str::<GithubAPI::Tree>(&text) {
                Ok(tree) => {
                    if tree.truncated {
                        return -1; // unlikely: >100k blobs or blob > 7MB
                    }

                    let total_size_bytes: i64 = tree.tree.iter().map(|file| file.size).sum();

                    // overflow unlikely: >2TB
                    return ((total_size_bytes + 512) / 1024) as i32;
                },
                Err(e) => {
                    Log::error("Failed to parse Github tree response: {}", e);
                    return -1;
                }
            }
        }

        -1
    }

    /// Query GitHub for topics named "unciv-mod*"
    ///
    /// # Returns
    ///
    /// Parsed `TopicSearchResponse` json on success, `None` on failure.
    pub fn try_get_github_topics() -> Option<GithubAPI::TopicSearchResponse> {
        let link = GithubAPI::url_to_query_mod_topics();
        let mut retries = 2;

        while retries > 0 {
            retries -= 1;
            // obey rate limit
            if RateLimit::wait_for_limit() {
                return None;
            }

            // try download
            let response = Self::download(&link, Some(|response| {
                if response.status() == StatusCode::FORBIDDEN ||
                   (response.status() == StatusCode::OK && retries == 1) {
                    // Pass the response headers to the rate limit handler so it can process the rate limit headers
                    RateLimit::notify_http_response(response);
                    retries += 1; // An extra retry so the 403 is ignored in the retry count
                }
            }))?;

            let text = String::from_utf8_lossy(&response);
            match serde_json::from_str::<GithubAPI::TopicSearchResponse>(&text) {
                Ok(result) => return Some(result),
                Err(e) => {
                    Log::error("Failed to parse Github topics response: {}", e);
                    return None;
                }
            }
        }

        None
    }

    /// Rewrite modOptions file for a mod we just installed to include metadata we got from the GitHub api
    ///
    /// (called on background thread)
    pub fn rewrite_mod_options(repo: &GithubAPI::Repo, mod_folder: &Path) -> io::Result<()> {
        let mod_options_file = mod_folder.join("jsons/ModOptions.json");
        let mod_options = if mod_options_file.exists() {
            from_json_file::<ModOptions>(&mod_options_file)?
        } else {
            ModOptions::default()
        };

        // If this is false we didn't get github repo info, do a defensive merge so the Repo.parseUrl or download
        // code can decide defaults but leave any meaningful field of a zip-included ModOptions alone.
        let overwrite_always = repo.direct_zip_url.is_empty();

        let mut updated_options = mod_options;

        if overwrite_always || updated_options.mod_url.is_empty() {
            updated_options.mod_url = repo.html_url.clone();
        }

        if overwrite_always || (updated_options.default_branch == "master" && !repo.default_branch.is_empty()) {
            updated_options.default_branch = repo.default_branch.clone();
        }

        if overwrite_always || updated_options.last_updated.is_empty() {
            updated_options.last_updated = repo.pushed_at.clone();
        }

        if overwrite_always || updated_options.author.is_empty() {
            updated_options.author = repo.owner.login.clone();
        }

        if overwrite_always || updated_options.mod_size == 0 {
            updated_options.mod_size = repo.size;
        }

        if overwrite_always || updated_options.topics.is_empty() {
            updated_options.topics = repo.topics.clone();
        }

        // Update deprecations if needed
        // updated_options.update_deprecations();

        // Write the updated options back to the file
        to_json(&updated_options, &mod_options_file)?;

        Ok(())
    }

    /// Convert a repository name to a local name for both display and folder name
    ///
    /// Replaces '-' with blanks but ensures no leading or trailing blanks.
    /// As mad modders know no limits, trailing "-" did indeed happen, causing things to break due to trailing blanks on a folder name.
    /// As "test-" and "test" are different allowed repository names, trimmed blanks are replaced with one equals sign per side.
    ///
    /// # Arguments
    ///
    /// * `only_outer_blanks` - If `true` ignores inner dashes - only start and end are treated. Useful when modders have manually created local folder names using dashes.
    pub fn repo_name_to_folder_name(name: &str, only_outer_blanks: bool) -> String {
        let mut result = if only_outer_blanks {
            name.to_string()
        } else {
            name.replace('-', " ")
        };

        if result.ends_with(' ') {
            result = result.trim_end().to_string() + &Self::OUTER_BLANK_REPLACEMENT.to_string();
        }

        if result.starts_with(' ') {
            result = Self::OUTER_BLANK_REPLACEMENT.to_string() + &result.trim_start();
        }

        result
    }

    /// Inverse of `repo_name_to_folder_name`
    pub fn folder_name_to_repo_name(name: &str) -> String {
        let mut result = name.replace(' ', "-");

        if result.ends_with(Self::OUTER_BLANK_REPLACEMENT) {
            result = result.trim_end_matches(Self::OUTER_BLANK_REPLACEMENT).to_string() + "-";
        }

        if result.starts_with(Self::OUTER_BLANK_REPLACEMENT) {
            result = "-".to_string() + &result.trim_start_matches(Self::OUTER_BLANK_REPLACEMENT);
        }

        result
    }
}

/// Extension trait for String to add repository name conversion methods
pub trait RepoNameExt {
    fn repo_name_to_folder_name(&self, only_outer_blanks: bool) -> String;
    fn folder_name_to_repo_name(&self) -> String;
}

impl RepoNameExt for str {
    fn repo_name_to_folder_name(&self, only_outer_blanks: bool) -> String {
        Github::repo_name_to_folder_name(self, only_outer_blanks)
    }

    fn folder_name_to_repo_name(&self) -> String {
        Github::folder_name_to_repo_name(self)
    }
}

/// Extension trait for Path to add rename or move functionality
pub trait PathExt {
    fn rename_or_move(&self, dest: &Path) -> io::Result<()>;
}

impl PathExt for Path {
    fn rename_or_move(&self, dest: &Path) -> io::Result<()> {
        // In Rust, we don't have the same file type distinction as in Kotlin
        // We'll just try to rename the file/directory
        if self.is_dir() {
            let target = dest.join(self.file_name().unwrap_or_default());
            fs::rename(self, target)?;
        } else {
            fs::rename(self, dest)?;
        }
        Ok(())
    }
}

/// Rate limit handling for GitHub API
pub struct RateLimit {
    /// When the rate limit will reset
    reset_time: Arc<Mutex<Instant>>,
    /// The number of remaining requests
    remaining: Arc<AtomicU64>,
    /// The total number of requests allowed
    limit: Arc<AtomicU64>,
}

impl RateLimit {
    /// Creates a new RateLimit
    pub fn new() -> Self {
        Self {
            reset_time: Arc::new(Mutex::new(Instant::now())),
            remaining: Arc::new(AtomicU64::new(60)),
            limit: Arc::new(AtomicU64::new(60)),
        }
    }

    /// Waits for the rate limit to reset if necessary
    ///
    /// # Returns
    ///
    /// `true` if we're still rate limited, `false` otherwise
    pub fn wait_for_limit() -> bool {
        let rate_limit = RATE_LIMIT.clone();
        let now = Instant::now();

        // Check if we need to wait
        let reset_time = *rate_limit.reset_time.lock().unwrap();
        if now < reset_time {
            let wait_time = reset_time.duration_since(now);
            thread::sleep(wait_time);
            return false;
        }

        // Check if we have remaining requests
        let remaining = rate_limit.remaining.load(Ordering::Relaxed);
        if remaining == 0 {
            // We're rate limited, wait for reset
            let reset_time = *rate_limit.reset_time.lock().unwrap();
            if now < reset_time {
                let wait_time = reset_time.duration_since(now);
                thread::sleep(wait_time);
            }
            return false;
        }

        // We have remaining requests
        rate_limit.remaining.fetch_sub(1, Ordering::Relaxed);
        false
    }

    /// Updates the rate limit information from HTTP response headers
    pub fn notify_http_response(response: &reqwest::Response) {
        let rate_limit = RATE_LIMIT.clone();

        // Get rate limit headers
        if let Some(limit) = response.headers().get("X-RateLimit-Limit") {
            if let Ok(limit_val) = limit.to_str().unwrap_or("60").parse::<u64>() {
                rate_limit.limit.store(limit_val, Ordering::Relaxed);
            }
        }

        if let Some(remaining) = response.headers().get("X-RateLimit-Remaining") {
            if let Ok(remaining_val) = remaining.to_str().unwrap_or("0").parse::<u64>() {
                rate_limit.remaining.store(remaining_val, Ordering::Relaxed);
            }
        }

        if let Some(reset) = response.headers().get("X-RateLimit-Reset") {
            if let Ok(reset_secs) = reset.to_str().unwrap_or("0").parse::<u64>() {
                let reset_time = Instant::now() + Duration::from_secs(reset_secs);
                *rate_limit.reset_time.lock().unwrap() = reset_time;
            }
        }
    }
}

/// Global rate limit instance
lazy_static::lazy_static! {
    static ref RATE_LIMIT: Arc<RateLimit> = Arc::new(RateLimit::new());
}

/// GitHub API types and functions
pub mod GithubAPI {
    use serde::{Deserialize, Serialize};

    /// GitHub repository information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Repo {
        pub name: String,
        pub html_url: String,
        pub default_branch: String,
        pub direct_zip_url: String,
        pub pushed_at: String,
        pub size: i32,
        pub owner: Owner,
        pub topics: Vec<String>,
    }

    /// GitHub repository owner information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Owner {
        pub login: String,
    }

    /// GitHub repository search results
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RepoSearch {
        pub total_count: i32,
        pub items: Vec<Repo>,
    }

    /// GitHub topic search results
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TopicSearchResponse {
        pub total_count: i32,
        pub items: Vec<Topic>,
    }

    /// GitHub topic information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Topic {
        pub name: String,
        pub score: f64,
    }

    /// GitHub tree information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Tree {
        pub truncated: bool,
        pub tree: Vec<TreeItem>,
    }

    /// GitHub tree item information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TreeItem {
        pub path: String,
        pub size: i64,
    }

    impl Repo {
        /// Gets the URL for querying the repository tree
        pub fn get_url_for_tree_query(&self) -> String {
            get_url_for_tree_query(self)
        }
    }

    /// Gets the URL for downloading a branch as a zip file
    pub fn get_url_for_branch_zip(repo_url: &str, branch: &str) -> String {
        format!("{}/archive/{}.zip", repo_url, branch)
    }

    /// Gets the URL for querying the repository tree
    pub fn get_url_for_tree_query(repo: &Repo) -> String {
        format!("https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
                repo.owner.login, repo.name, repo.default_branch)
    }

    /// Gets the URL for querying mod repositories
    pub fn get_url_for_mod_listing(search_request: &str, amount_per_page: i32, page: i32) -> String {
        let query = if search_request.is_empty() {
            "topic:unciv-mod".to_string()
        } else {
            format!("topic:unciv-mod {}", search_request)
        };

        format!("https://api.github.com/search/repositories?q={}&sort=updated&order=desc&per_page={}&page={}",
                query, amount_per_page, page)
    }

    /// Gets the URL for querying mod topics
    pub fn url_to_query_mod_topics() -> String {
        "https://api.github.com/search/topics?q=unciv-mod&sort=score&order=desc".to_string()
    }

    /// Gets the URL for the preview image
    pub fn get_url_for_preview(repo_url: &str, branch: &str) -> String {
        // Replace github.com with raw.githubusercontent.com and add the branch
        repo_url.replace("github.com", "raw.githubusercontent.com") + "/" + branch + "/preview"
    }
}

/// Extension trait for String to add hash method
pub trait HashExt {
    fn hash(&self) -> u64;
}

impl HashExt for str {
    fn hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// Extension trait for String to add toString method
pub trait ToStringExt {
    fn to_string(&self) -> String;
}

impl ToStringExt for Repo {
    fn to_string(&self) -> String {
        format!("{}:{}", self.owner.login, self.name)
    }
}