mod result;

use image::ImageReader;
pub use result::{Error, Result};

use std::{
    cell::Cell,
    collections::HashMap,
    ffi::OsStr,
    fs::{self},
    ops::Deref,
    path::Path,
    rc::Rc,
    time::Instant,
};

#[derive(Debug)]
pub enum ResourceData {
    Image {
        name: String,
        width: u32,
        height: u32,
        buffer: Vec<u8>,
    },
    Text {
        value: String,
    },
}

#[derive(Debug)]
struct ResourceDescriptor {
    _idle_cicles: Cell<usize>,
    _idle_since: Cell<Instant>,
    data: ResourceData,
}

#[derive(Clone, Debug)]
pub struct Resource {
    inner: Rc<ResourceDescriptor>,
}

impl Deref for Resource {
    type Target = ResourceData;
    fn deref(&self) -> &Self::Target {
        &self.inner.data
    }
}

pub struct ResourceManager {
    base_path: &'static str,
    _used_memory: usize,
    loaded_resources: HashMap<String, Resource>,
}

impl Resource {
    fn new(data: ResourceData) -> Self {
        Self {
            inner: Rc::new({
                ResourceDescriptor {
                    _idle_cicles: 0.into(),
                    _idle_since: Instant::now().into(),
                    data,
                }
            }),
        }
    }
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

                Ok(Resource::new(ResourceData::Image {
                    name: String::from(path),
                    width,
                    height,
                    buffer,
                }))
            }
            _ => Err(Error::UnsupportedExtension),
        }
    }
}
