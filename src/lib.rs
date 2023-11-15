//! ureq multipart post toolkit
//!
//! fork from [multipart](https://crates.io/crates/multipart)
//!
//!
//! # Examples 1
//!
//! ```ignore
//!
//! use ureq_multipart::MultipartBuilder;
//!
//! let (content_type,data) = MultipartBuilder::new()
//!             .add_file("test","1.txt").unwrap()
//!             .add_text("name","value")
//!             .finish().unwrap();
//! let resp: Value = ureq::post("http://some.service.url")
//!             .set("Content-Type", &content_type)
//!             .send_bytes(&data)?
//!             .into_json()?
//!
//! ```
//!
//! # Examples 2
//!
//! ```ignore
//!
//! use ureq_multipart::MultipartRequest;
//!
//! let resp: Value = ureq::post("http://some.service.url")
//!             .send_multipart_file("name","1.txt")?
//!             .into_json()?
//!
//! ```
use mime::Mime;
use rand::Rng;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use ureq::{Error, Request, Response};

use std::path::Path;

const BOUNDARY_LEN: usize = 29;

fn opt_filename(path: &Path) -> Option<&str> {
    path.file_name().and_then(|filename| filename.to_str())
}

fn random_alphanumeric(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Uniform::from(0..=9))
        .take(len)
        .map(|num| num.to_string())
        .collect()
}

fn mime_filename(path: &Path) -> (Mime, Option<&str>) {
    let content_type = mime_guess::from_path(path);
    let filename = opt_filename(path);
    (content_type.first_or_octet_stream(), filename)
}

/// multipart data build
#[derive(Debug)]
pub struct MultipartBuilder {
    boundary: String,
    inner: Vec<u8>,
    data_written: bool,
}
impl Default for MultipartBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MultipartBuilder {
    pub fn new() -> Self {
        Self {
            boundary: random_alphanumeric(BOUNDARY_LEN),
            inner: Vec::new(),
            data_written: false,
        }
    }
    /// add text field
    ///
    /// * name field name
    /// * text field text value
    pub fn add_text(mut self, name: &str, text: &str) -> io::Result<Self> {
        self.write_field_headers(name, None, None)?;
        self.inner.write_all(text.as_bytes())?;
        Ok(self)
    }
    /// add file
    ///
    /// * name file field name
    /// * path the sending file path
    pub fn add_file<P: AsRef<Path>>(self, name: &str, path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let (content_type, filename) = mime_filename(path);
        let mut file = File::open(path)?;
        self.add_stream(&mut file, name, filename, Some(content_type))
    }
    /// add some stream
    pub fn add_stream<S: Read>(
        mut self,
        stream: &mut S,
        name: &str,
        filename: Option<&str>,
        content_type: Option<Mime>,
    ) -> io::Result<Self> {
        // This is necessary to make sure it is interpreted as a file on the server end.
        let content_type = Some(content_type.unwrap_or(mime::APPLICATION_OCTET_STREAM));
        self.write_field_headers(name, filename, content_type)?;
        io::copy(stream, &mut self.inner)?;
        Ok(self)
    }
    fn write_boundary(&mut self) -> io::Result<()> {
        if self.data_written {
            self.inner.write_all(b"\r\n")?;
        }

        write!(
            self.inner,
            "-----------------------------{}\r\n",
            self.boundary
        )
    }
    fn write_field_headers(
        &mut self,
        name: &str,
        filename: Option<&str>,
        content_type: Option<Mime>,
    ) -> io::Result<()> {
        self.write_boundary()?;
        if !self.data_written {
            self.data_written = true;
        }
        write!(
            self.inner,
            "Content-Disposition: form-data; name=\"{name}\""
        )?;
        if let Some(filename) = filename {
            write!(self.inner, "; filename=\"{filename}\"")?;
        }
        if let Some(content_type) = content_type {
            write!(self.inner, "\r\nContent-Type: {content_type}")?;
        }
        self.inner.write_all(b"\r\n\r\n")
    }
    /// general multipart data
    ///
    /// # Return
    /// * (content_type,post_data)
    ///    * content_type http header content type
    ///    * post_data ureq.req.send_send_bytes(&post_data)
    ///
    pub fn finish(mut self) -> io::Result<(String, Vec<u8>)> {
        if self.data_written {
            self.inner.write_all(b"\r\n")?;
        }

        // always write the closing boundary, even for empty bodies
        write!(
            self.inner,
            "-----------------------------{}--\r\n",
            self.boundary
        )?;
        Ok((
            format!(
                "multipart/form-data; boundary=---------------------------{}",
                self.boundary
            ),
            self.inner,
        ))
    }
}

/// multipart request for ureq
/// add send_multipart_file/send_multipart_files method to ureq Request
pub trait MultipartRequest {
    fn send_multipart_files<P: AsRef<Path>>(self, files: &[P]) -> Result<Response, Error>;
    fn send_multipart_file<P: AsRef<Path>>(self, name: &str, file: P) -> Result<Response, Error>;
}
impl MultipartRequest for Request {
    /// send multi files,auto set the name with file's name by multipart
    fn send_multipart_files<P: AsRef<Path>>(self, files: &[P]) -> Result<Response, Error> {
        let mut builder = MultipartBuilder::new();
        for file_path in files {
            let file_path = file_path.as_ref();
            let file_name = file_path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();
            builder = builder.add_file(file_name, file_path)?;
        }
        let (content_type, data) = builder.finish()?;
        self.set("Content-Type", &content_type).send_bytes(&data)
    }
    /// send single file with name by multipart
    fn send_multipart_file<P: AsRef<Path>>(self, name: &str, path: P) -> Result<Response, Error> {
        let (content_type, data) = MultipartBuilder::new().add_file(name, path)?.finish()?;
        self.set("Content-Type", &content_type).send_bytes(&data)
    }
}
#[cfg(test)]
mod test {
    use super::*;

    fn get_file_string(p: &Path) -> String {
        let mut file = File::open(p).unwrap();
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content).unwrap();
        String::from_utf8(file_content.clone()).unwrap()
    }

    #[test]
    fn test_builder() {
        let p = Path::new("test-vector0.txt");
        let file_str = get_file_string(p);

        let (content_type, data) = MultipartBuilder::new()
            .add_file("test", p)
            .unwrap()
            .finish()
            .unwrap();

        assert!(data.len() > 0);
        assert!(content_type.contains("multipart/form-data;"));
        let datastr = String::from_utf8(data.clone()).unwrap();
        assert!(datastr.contains(&file_str));
    }

    #[test]
    fn test_ureq() {
        let p0 = Path::new("test-vector0.txt");
        let p1 = Path::new("test-vector1.txt");
        let file0_str = get_file_string(p0);
        let file1_str = get_file_string(p0);

        let resp = ureq::post("https://httpbin.org/anything")
            .send_multipart_file("name", p0)
            .unwrap();
        assert!(resp.status() == 200);
        let body = resp.into_string().unwrap();
        assert!(body.contains(&file0_str));

        let resp = ureq::post("https://httpbin.org/anything")
            .send_multipart_files(&[p0, p1])
            .unwrap();
        assert!(resp.status() == 200);
        let body = resp.into_string().unwrap();
        println!("body {:?}", body);
        assert!(body.contains(&file0_str));
        assert!(body.contains(&file1_str));
    }
}
