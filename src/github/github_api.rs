use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;

/// GitHub API module
///
/// This module collects all GitHub API structural knowledge:
/// - Response schema
/// - Query URL builders
///
/// Collected doc links:
/// - https://docs.github.com/en/repositories/working-with-files/using-files/downloading-source-code-archives#source-code-archive-urls
/// - https://docs.github.com/en/rest/reference/search#search-repositories--code-samples
/// - https://docs.github.com/en/rest/repos/repos
/// - https://docs.github.com/en/rest/releases/releases
/// - https://docs.github.com/en/rest/git/trees#get-a-tree
pub mod GithubAPI {
    use super::*;

    /// Format a download URL for a branch archive
    ///
    /// URL format see: https://docs.github.com/en/repositories/working-with-files/using-files/downloading-source-code-archives#source-code-archive-urls
    /// Note: https://api.github.com/repos/owner/mod/zipball would be an alternative. Its response is a redirect, but our lib follows that and delivers the zip just fine.
    /// Problems with the latter: Internal zip structure different, finalDestinationName would need a patch. Plus, normal URL escaping for owner/reponame does not work.
    pub fn get_url_for_branch_zip(git_repo_url: &str, branch: &str) -> String {
        format!("{}/archive/refs/heads/{}.zip", git_repo_url, branch)
    }

    /// Format a URL to query for Mod repos by topic
    pub fn get_url_for_mod_listing(search_request: &str, amount_per_page: i32, page: i32) -> String {
        // Add + if needed to separate the query text from its parameters
        let search_part = if search_request.is_empty() {
            String::new()
        } else {
            format!("{} +", search_request)
        };

        format!("https://api.github.com/search/repositories?q={}%20topic:unciv-mod%20fork:true&sort:stars&per_page={}&page={}",
                search_part, amount_per_page, page)
    }

    /// Format URL to fetch one specific [Repo] metadata from the API
    pub fn get_url_for_single_repo_query(owner: &str, repo_name: &str) -> String {
        format!("https://api.github.com/repos/{}/{}", owner, repo_name)
    }

    /// Format a download URL for a release archive
    pub fn get_url_for_release_zip(repo: &Repo) -> String {
        format!("{}/archive/refs/tags/{}.zip", repo.html_url, repo.release_tag)
    }

    /// Format a URL to query a repo tree - to calculate actual size
    /// It's hard to see in the doc this not only accepts a commit SHA, but either branch (used here) or tag names too
    pub fn get_url_for_tree_query(repo: &Repo) -> String {
        format!("https://api.github.com/repos/{}/git/trees/{}?recursive=true",
                repo.full_name, repo.default_branch)
    }

    /// Format a URL to fetch a preview image - without extension
    pub fn get_url_for_preview(mod_url: &str, branch: &str) -> String {
        format!("{}/{}/preview", mod_url, branch)
            .replace("github.com", "raw.githubusercontent.com")
    }

    /// A query returning all known topics staring with "unciv-mod" and having at least two uses
    /// `+repositories:>1` means ignore unused or practically unused topics
    pub const URL_TO_QUERY_MOD_TOPICS: &str = "https://api.github.com/search/topics?q=unciv-mod+repositories:%3E1&sort=name&order=asc";

    /// Parsed Github repo search response
    ///
    /// # Fields
    ///
    /// * `total_count` - Total number of hits for the search (ignoring paging window)
    /// * `incomplete_results` - A flag set by github to indicate search was incomplete (never seen it on)
    /// * `items` - Array of repositories
    ///
    /// See: https://docs.github.com/en/rest/reference/search#search-repositories--code-samples
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RepoSearch {
        #[serde(rename = "total_count")]
        pub total_count: i32,
        #[serde(rename = "incomplete_results")]
        pub incomplete_results: bool,
        pub items: Vec<Repo>,
    }

    /// Part of [RepoSearch] in Github API response - one repository entry in [items][RepoSearch.items]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Repo {
        /// Unlike the rest of this class, this is not part of the API but added by us locally
        /// to track whether [getRepoSize][Github.getRepoSize] has been run successfully for this repo
        #[serde(skip)]
        pub has_updated_size: bool,

        /// Not part of the github schema: Explicit final zip download URL for non-github or release downloads
        #[serde(skip)]
        pub direct_zip_url: String,

        /// Not part of the github schema: release tag, for debugging (DL via direct_zip_url)
        #[serde(skip)]
        pub release_tag: String,

        pub name: String,
        #[serde(rename = "full_name")]
        pub full_name: String,
        pub description: Option<String>,
        pub owner: RepoOwner,
        #[serde(rename = "stargazers_count")]
        pub stargazers_count: i32,
        #[serde(rename = "default_branch")]
        pub default_branch: String,
        #[serde(rename = "html_url")]
        pub html_url: String,
        #[serde(rename = "pushed_at")]
        pub pushed_at: String, // don't use updated_at - see https://github.com/yairm210/Unciv/issues/6106
        pub size: i32,
        pub topics: Vec<String>,
        // pub stargazers_url: String,
        // pub homepage: Option<String>,      // might use instead of go to repo?
        // pub has_wiki: bool,                // a wiki could mean proper documentation for the mod?

        /// String representation to be used for logging
        pub fn to_string(&self) -> String {
            if self.name.is_empty() {
                self.direct_zip_url.clone()
            } else {
                self.name.clone()
            }
        }

        /// Create a [Repo] metadata instance from a [url], supporting various formats
        /// from a repository landing page url to a free non-github zip download.
        ///
        /// See: GithubAPI.parseUrl
        /// Returns `None` for invalid links or any other failures
        pub fn parse_url(url: &str) -> Option<Repo> {
            let mut repo = Repo {
                has_updated_size: false,
                direct_zip_url: String::new(),
                release_tag: String::new(),
                name: String::new(),
                full_name: String::new(),
                description: None,
                owner: RepoOwner {
                    login: String::new(),
                    avatar_url: None,
                },
                stargazers_count: 0,
                default_branch: "master".to_string(),
                html_url: url.to_string(),
                pushed_at: String::new(),
                size: 0,
                topics: Vec::new(),
            };

            repo.parse_url(url)
        }

        /// Query Github API for [owner]'s [repo_name] repository metadata
        pub fn query(owner: &str, repo_name: &str) -> Option<Repo> {
            // This would be implemented in the actual code to make an HTTP request
            // For now, we'll just return None as a placeholder
            None
        }

        /// Initialize `this` with an url, extracting all possible fields from it
        /// (html_url, author, repoName, branchName).
        ///
        /// Allow url formats:
        /// * Basic repo url:
        ///   https://github.com/author/repoName
        /// * or complete 'zip' url from github's code->download zip menu:
        ///   https://github.com/author/repoName/archive/refs/heads/branchName.zip
        /// * or the branch url same as one navigates to on github through the "branches" menu:
        ///   https://github.com/author/repoName/tree/branchName
        /// * or release tag
        ///   https://github.com/author/repoName/releases/tag/tagname
        ///   https://github.com/author/repoName/archive/refs/tags/tagname.zip
        ///
        /// In the case of the basic repo url, an [API query](https://docs.github.com/en/rest/repos/repos#get-a-repository) is sent to determine the default branch.
        /// Other url forms will not go online.
        ///
        /// Returns a new Repo instance for the 'Basic repo url' case, otherwise `this`, modified, to allow chaining, `None` for invalid links or any other failures
        /// See: https://docs.github.com/en/rest/repos/repos#get-a-repository--code-samples
        fn parse_url(&mut self, url: &str) -> Option<Repo> {
            // This would be implemented in the actual code to parse URLs
            // For now, we'll just return None as a placeholder
            None
        }
    }

    /// Part of [Repo] in Github API response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RepoOwner {
        pub login: String,
        #[serde(rename = "avatar_url")]
        pub avatar_url: Option<String>,
    }

    /// Topic search response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TopicSearchResponse {
        // Commented out: Github returns them, but we're not interested
        // pub total_count: i32,
        // pub incomplete_results: bool,
        pub items: Vec<Topic>,
    }

    /// Topic information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Topic {
        pub name: String,
        #[serde(rename = "display_name")]
        pub display_name: Option<String>,  // Would need to be curated, which is a lot of work
        // pub featured: bool,
        // pub curated: bool,
        #[serde(rename = "created_at")]
        pub created_at: String, // iso datetime with "Z" timezone
        #[serde(rename = "updated_at")]
        pub updated_at: String, // iso datetime with "Z" timezone
    }

    /// Class to receive a github API "Get a tree" response parsed as json
    /// Parts of the response we ignore are commented out
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Tree {
        // pub sha: String,
        // pub url: String,

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct TreeFile {
            // pub path: String,
            // pub mode: i32,
            // pub r#type: String, // blob / tree
            // pub sha: String,
            // pub url: String,
            pub size: i64,
        }

        pub tree: Vec<TreeFile>,
        pub truncated: bool,
    }
}