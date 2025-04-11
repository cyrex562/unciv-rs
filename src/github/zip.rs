use std::fs::{self, File};
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use zip::{ZipArchive, ZipFile, ZipEntry};
use log::debug;

/// Utility for extracting ZIP archives
/// See: [extract_folder]
pub struct Zip;

impl Zip {
    /// Buffer size for reading and writing files
    const BUFFER_SIZE: usize = 2048;

    /// Extract one Zip file recursively (nested Zip files are extracted in turn).
    ///
    /// The source Zip is not deleted, but successfully extracted nested ones are.
    ///
    /// **Warning**: Extracting into a non-empty destination folder will merge contents. Existing
    /// files also included in the archive will be partially overwritten, when the new data is shorter
    /// than the old you will get _mixed contents!_
    ///
    /// # Arguments
    ///
    /// * `zip_path` - The path to the Zip file to extract
    /// * `destination_path` - The folder to extract into, preferably empty (not enforced)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if extraction was successful
    /// * `Err(io::Error)` if an error occurred during extraction
    pub fn extract_folder<P: AsRef<Path>, Q: AsRef<Path>>(
        zip_path: P,
        destination_path: Q,
    ) -> io::Result<()> {
        let zip_path = zip_path.as_ref();
        let destination_path = destination_path.as_ref();

        debug!("Extracting {:?} to {:?}", zip_path, destination_path);

        // Open the zip file
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        // Process each entry in the zip file
        for i in 0..archive.len() {
            let mut zip_entry = archive.by_index(i)?;
            let entry_path = zip_entry.name().to_string();
            let dest_path = destination_path.join(&entry_path);

            // Create parent directories if they don't exist
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Extract the file
            if !zip_entry.is_dir() {
                Self::extract_file(&mut zip_entry, &dest_path)?;

                // If this is a zip file, extract it recursively
                if entry_path.ends_with(".zip") {
                    Self::extract_folder(&dest_path, dest_path.parent().unwrap())?;
                    fs::remove_file(dest_path)?;
                }
            }
        }

        Ok(())
    }

    /// Extract a single file from a zip entry
    fn extract_file(zip_entry: &mut ZipFile, dest_path: &Path) -> io::Result<()> {
        // Create the destination file
        let dest_file = File::create(dest_path)?;
        let mut writer = BufWriter::with_capacity(Self::BUFFER_SIZE, dest_file);
        let mut buffer = vec![0; Self::BUFFER_SIZE];

        // Read and write until the end of the file
        loop {
            let bytes_read = zip_entry.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            writer.write_all(&buffer[..bytes_read])?;
        }

        writer.flush()?;
        Ok(())
    }
}