use core::any::{Any, TypeId};

use hashbrown::HashMap;

#[derive(Default, Clone)]
pub struct AnyMap {
    inner: HashMap<TypeId, Box<dyn AnyClone + Send + Sync + 'static>>,
}

impl Clone for Box<dyn AnyClone + Send + Sync + 'static> {
    fn clone(&self) -> Self {
        (**self).clone_boxed()
    }
}

pub trait AnyClone: Any {
    fn clone_boxed(&self) -> Box<dyn AnyClone + Send + Sync>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any + Clone + Send + Sync + 'static> AnyClone for T {
    fn clone_boxed(&self) -> Box<dyn AnyClone + Send + Sync> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl AnyMap {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        dbg!(core::any::type_name::<T>());
        self.inner
            .get(&TypeId::of::<T>())
            .and_then(|b| b.as_ref().as_any().downcast_ref::<T>())
    }

    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.inner
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.as_any_mut().downcast_mut())
    }

    pub fn insert<T: AnyClone + Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        dbg!(core::any::type_name::<T>());
        self.inner
            .insert(TypeId::of::<T>(), Box::new(value))
            .and_then(|b| b.into_any().downcast::<T>().ok())
            .map(|b| *b)
    }

    pub fn remove<T: Any + Send>(&mut self) -> Option<T> {
        dbg!(core::any::type_name::<T>());
        self.inner
            .remove(&TypeId::of::<T>())
            .and_then(|b| b.into_any().downcast::<T>().ok())
            .map(|b| *b)
    }
}

#[test]
fn test_anymap() {
    let mut map = AnyMap::new();

    assert!(map.insert(1i32).is_none());
    assert!(map.insert(2i32).is_some_and(|i| i == 1));

    assert_eq!(*map.get::<i32>().unwrap(), 2);

    assert!(map.insert(String::from("hello, world!")).is_none());
    assert!(map.get::<String>().is_some_and(|s| s == "hello, world!"));

    assert!(map.remove::<String>().is_some_and(|s| s == "hello, world!"));
}

#[test]
fn anymap_clone() {
    let mut map = AnyMap::new();

    map.insert(String::from("hi"));
    let _ = map.clone();
}
