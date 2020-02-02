pub mod health;
pub mod api;

#[cfg(test)]
mod tests {
    use iron::response::WriteBody;
    use std::io::{ self, Write };
    use serde::de;
    use iron::IronError;

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

    pub fn parse_body<T: de::DeserializeOwned>(body: Option<Box<dyn WriteBody>>) -> Result<T, String> {
        let stringified = stringify_body(body);
        let deserialized = serde_json::from_str(&*stringified);
        match deserialized {
            Err(e) => {
                Err(format!("{:?}", e))
            }
            Ok(value) => {
                Ok(value)
            }
        }
    }

    pub fn iron_error_translate<T>(result: Result<T, IronError>) -> Result<T, String> {
        match result {
            Err(e) => {
                Err(format!("{:?}", e))
            },
            Ok(value) => {
                Ok(value)
            },
        }
    }
}