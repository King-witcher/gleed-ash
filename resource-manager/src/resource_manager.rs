use std::collections::HashMap;
use std::fs;
use std::path::Path;

use image::ImageReader;

use crate::resource::ImageResource;
use crate::result::Result;
use crate::{Error, Resource, ResourceData};

pub struct ResourceManager {
    base_path: &'static str,
    _used_memory: usize,
    loaded_resources: HashMap<String, Resource>,
}

impl ResourceManager {
    pub fn new(base_path: &'static str) -> Self {
        Self {
            base_path,
            _used_memory: 0,
            loaded_resources: HashMap::new(),
        }
    }

    pub fn get(&mut self, path: &str) -> Result<Resource> {
        match self.loaded_resources.get(path) {
            Some(resource) => Ok(resource.clone()),
            None => {
                let res = self.load_from_fs(path)?;
                self.loaded_resources.insert(path.into(), res.clone());
                Ok(res)
            }
        }
    }

    fn load_from_fs(&self, path: &str) -> Result<Resource> {
        let path_obj = Path::new(self.base_path).join(path);
        let path_obj = path_obj.as_path();

        let metadata = fs::metadata(path_obj)?;
        if !metadata.is_file() {
            return Err(Error::ResourceNotFound);
        }

        match path_obj.extension() {
            Some(ext) if ext == "png" || ext == "bmp" => {
                let image = ImageReader::open(path_obj)?.decode()?.to_rgba8();
                let width = image.width();
                let height = image.height();
                let buffer = image.into_raw();

                let image_resource = ImageResource {
                    buffer,
                    height,
                    width,
                };

                Ok(ResourceData::Image(image_resource).into())
            }
            _ => Err(Error::UnsupportedExtension),
        }
    }
}
