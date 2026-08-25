use std::{cell::Cell, fmt::Debug, ops::Deref, rc::Rc, time::Instant};

pub struct ImageResource {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>,
}

impl Debug for ImageResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{} image", self.width, self.height)
    }
}

impl ImageResource {
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Debug)]
pub enum ResourceData {
    Image(ImageResource),
    Text(String),
}

/// Auxiliar structure that describes a resource and metadata about when this resource can be released
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

impl From<ResourceData> for Resource {
    fn from(data: ResourceData) -> Self {
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

impl Deref for Resource {
    type Target = ResourceData;
    fn deref(&self) -> &Self::Target {
        &self.inner.data
    }
}
