extern crate phf;

#[cfg(feature = "flate2")]
extern crate flate2;

use std::borrow::{Borrow, Cow};
use std::io::{self, BufReader, Cursor, Error, ErrorKind, Read};
use std::fs::File;

#[cfg(feature = "flate2")]
use flate2::bufread::GzDecoder;

pub enum Compression {
    None,
    Gzip,
    Passthrough,
}

/// Runtime access to the included files
pub struct Files {
    /// **Do not access this field, it is only public to allow for code generation!**
    pub files: phf::Map<&'static str, (Compression, &'static [u8])>,
}

#[cfg(windows)]
fn as_key(path: &str) -> Cow<str> {
    Cow::Owned(path.replace("\\", "/"))
}

#[cfg(not(windows))]
fn as_key(path: &str) -> Cow<str> {
    Cow::Borrowed(path)
}

impl Files {
    pub fn available(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Returns an iterator over all available file names.  Does not
    /// decompress any compressed data.
    pub fn file_names(&'static self) -> FileNames {
        FileNames { iter: self.files.keys() }
    }

    pub fn get(&self, path: &str) -> io::Result<Cow<'static, [u8]>> {
        let key = as_key(path);
        match self.files.get(key.borrow() as &str) {
            Some(b) => {
                match b.0 {
                    Compression::None => Ok(Cow::Borrowed(b.1)),
                    #[cfg(feature = "flate2")]
                    Compression::Gzip => {
                        let mut r = GzDecoder::new(b.1);
                        let mut v = Vec::new();
                        r.read_to_end(&mut v)?;
                        Ok(Cow::Owned(v))
                    }
                    #[cfg(not(feature = "flate2"))]
                    Compression::Gzip => panic!("Feature 'flate2' not enabled"),
                    Compression::Passthrough => {
                        let mut r = BufReader::new(File::open(path)?);
                        let mut v = Vec::new();
                        r.read_to_end(&mut v)?;
                        Ok(Cow::Owned(v))
                    }
                }
            }
            None => Err(Error::new(ErrorKind::NotFound, "Key not found")),
        }
    }

    pub fn read(&self, path: &str) -> io::Result<Box<Read>> {
        let key = as_key(path);
        match self.files.get(key.borrow() as &str) {
            Some(b) => {
                match b.0 {
                    Compression::None => Ok(Box::new(Cursor::new(b.1))),
                    #[cfg(feature = "flate2")]
                    Compression::Gzip => Ok(Box::new(GzDecoder::new(Cursor::new(b.1)))),
                    #[cfg(not(feature = "flate2"))]
                    Compression::Gzip => panic!("Feature 'flate2' not enabled"),
                    Compression::Passthrough => {
                        Ok(Box::new(BufReader::new(File::open(path)?)))
                    }
                }
            }
            None => Err(Error::new(ErrorKind::NotFound, "Key not found")),
        }
    }
}

/// Iterates over the file names available for `Files` object.
pub struct FileNames {
    // Our internal iterator.  We wrap this in a nice struct so our
    // caller doesn't need to know the details.
    iter: phf::map::Keys<'static, &'static str, (Compression, &'static [u8])>,
}

impl Iterator for FileNames {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|s| *s)
    }
}
