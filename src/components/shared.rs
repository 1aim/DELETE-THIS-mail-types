
use std::rc::Rc;
use owning_ref::OwningRef;
use std::ops::{ Deref, DerefMut };

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Item(OwningRef<Rc<String>, str>);

impl Item {

    pub fn new<S: Into<String>>(data: S) -> Item {
        Item( OwningRef::new( Rc::new( data.into() ) )
            .map( |str_rc| &**str_rc ) )
    }
}

impl<S> From<S> for Item where S: Into<String> {
    fn from(data: S) -> Self {
        Self::new( data )
    }
}

impl Deref for Item {
    type Target = OwningRef<Rc<String>, str>;

    fn deref( &self ) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Item {

    fn deref_mut( &mut self ) -> &mut Self::Target {
        &mut self.0
    }
}