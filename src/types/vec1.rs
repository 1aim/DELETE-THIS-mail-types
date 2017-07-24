
use std::ops::{ Deref, DerefMut };
use std::result::{ Result as StdResult };

type Size0Error = ();
type Vec1Result<T> = StdResult<T, Size0Error>;

pub struct Vec1<T>(Vec<T>);

impl<T> Vec1<T> {


    pub fn new( first: T  ) -> Self {
        Vec1( vec![ first ] )
    }

    pub fn new_with_capacity( first: T, capacity: usize ) -> Self {
        let mut vec = Vec::with_capacity( capacity );
        vec.push( first );
        Vec1( vec )
    }

    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve( additional )
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.0.reserve_exact( additional )
    }

    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit()
    }

    pub fn try_truncate(&mut self, len: usize) -> Vec1Result<()> {
        if len > 0 {
            self.0.truncate( len );
            Ok( () )
        } else {
            Err( Size0Error )
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.0.as_mut_slice()
    }

    pub fn try_swap_remove(&mut self, index: usize) -> Vec1Result<T> {
        if self.len() > 1 {
            Ok( self.swap_remove( index ) )
        } else {
            Err( Size0Error )
        }
    }

    pub fn insert(&mut self, index: usize, element: T) {
        self.0.insert( index, element )
    }

    pub fn try_remove( &mut self, index: usize ) -> Vec1Result<T> {
        if self.len() > 1 {
            Ok( self.0.remove( index ) )
        } else {
            Err( Size0Error )
        }
    }

    pub fn dedup_by_key<F, K>(&mut self, key: F)
        where F: FnMut(&mut T) -> K,
              K: PartialEq<K>
    {
        self.0.dedup_by_key( key )
    }

    pub fn dedup_by<F>(&mut self, same_bucket: F)
        where F: FnMut(&mut T, &mut T) -> bool
    {
        self.0.dedup_by( same_bucket )
    }

    pub fn push(&mut self, value: T) {
        self.0.push( value )
    }

    /// pops if there is _more_ than 1 element in the vector
    pub fn pop(&mut self) -> Option<T> {
        if self.len() > 1 {
            self.0.pop()
        } else {
            None
        }
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        self.0.append( other )
    }

    pub fn extend_from_slice(&mut self, other: &[T]) {
        self.0.extend_from_slice( other )
    }
}


impl<T> Vec1<T> where T: Clone {
    pub fn try_resize(&mut self, new_len: usize, value: T) -> Vec1Result<()> {
        if new_len >= 1 {
            Ok( self.0.resize( new_len, value ) )
        } else {
            Err( Size0Error )
        }
    }
}


impl<T> Vec1<T> where T: PartialEq<T> {
    pub fn dedup(&mut self) {
        self.0.dedup()
    }
}


impl<T> Deref for Vec1<T> {
    type Target = Vec<T>;

    fn deref( &self ) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Vec1<T> {
    fn deref_mut( &self ) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Into<Vec<T>> for Vec1<T> {

    fn into( self ) -> Vec<T> {
        self.0
    }
}
