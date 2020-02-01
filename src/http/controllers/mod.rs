pub mod health;
pub mod api;

#[cfg(test)]
mod tests {
    use iron::response::WriteBody;
    use std::io::{ self, Write };

    struct WriteProxy {
        all_data: Vec<u8>,
    }

    impl Write for WriteProxy {
        fn write(&mut self, data: &[u8]) -> Result<usize, io::Error> {
            self.all_data.append(&mut data.to_vec());
            Ok(data.len())
        }

        fn flush(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
    }

    pub fn stringify_body(body: Option<Box<dyn WriteBody>>) -> String {
        let mut write_proxy = WriteProxy {
            all_data: vec![]
        };
        match body {
            Some(mut write_body) => {
                write_body.write_body(&mut write_proxy).unwrap();
            }
            None => ()
        }
        return (&*String::from_utf8_lossy(&write_proxy.all_data)).to_string();
    }
}