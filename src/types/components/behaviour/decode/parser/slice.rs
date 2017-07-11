use std::ops::{ Range, RangeFrom, RangeTo, RangeFull };
use nom::*;
use nom::{ Slice as NomSlice };
use super::*;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Slice<'a> {
    current: &'a str,
    base_offset: usize
}

impl<'a> Slice<'a> {
    pub fn new( base: &'a str ) -> Slice<'a> {
        Slice {
            current: base,
            base_offset: 0
        }
    }

    pub fn as_base_range( &self ) -> Range<usize> {
        Range { start: self.base_offset, end: self.base_offset + self.current.len() }
    }

    // we implement nearly the same interface as Take,
    // as we can't implement take as it's design is incompatible with
    // manged slices (it returns &Self on split, but the & is part of the type
    // in managed slices)
    // Luckily this is used in macros only, so as long as they are _similar enough
    // for the macro expansion_ it's fine
    pub fn take(&self, count: usize)  -> Option<Self> {
        self.current.take::<()>( count ).map( |strslice| {
            Slice {
                current: strslice,
                base_offset: self.base_offset
            }
        })
    }
    pub fn take_split(&self, count: usize) -> Option<(Self,Self)> {
        self.current.take_split::<()>( count ).map( |(from_count, until_count)| {
            ( Slice { current: from_count, base_offset: self.base_offset + count },
              Slice { current: until_count, base_offset: self.base_offset } )
        })
    }
}


impl<'a> Offset for Slice<'a> {
    fn offset(&self, second: &Self) -> usize {
        let _1st = self.current.as_ptr();
        let _2nd = second.current.as_ptr();

        (_1st as usize) - (_2nd as usize)
    }
}




impl<'a> InputIter for Slice<'a> {
    type Item = <&'a str as InputIter>::Item;
    type RawItem = <&'a str as InputIter>::RawItem;
    type Iter = <&'a str as InputIter>::Iter;
    type IterElem = <&'a str as InputIter>::IterElem;

    #[inline(always)]
    fn iter_indices(&self)  -> Self::Iter {
        self.current.iter_indices()
    }

    #[inline(always)]
    fn iter_elements(&self) -> Self::IterElem {
        self.current.iter_elements()
    }

    #[inline(always)]
    fn position<P>(&self, predicate: P) -> Option<usize> where P: Fn(Self::RawItem) -> bool {
        self.current.position( predicate )
    }

    #[inline(always)]
    fn slice_index(&self, count: usize) -> Option<usize> {
        self.current.slice_index( count )
    }
}

impl<'a> InputLength for Slice<'a> {
    #[inline(always)]
    fn input_len(&self) -> usize {
        self.current.input_len()
    }
}



impl<'a, T> Compare<T> for Slice<'a> where &'a str: Compare<T> {

    fn compare(&self, t: T) -> CompareResult {
        self.current.compare( t )
    }

    fn compare_no_case(&self, t: T) -> CompareResult {
        self.current.compare_no_case( t )
    }
}

impl<'a> FindToken<Slice<'a>> for u8 {
    fn find_token(&self, input: Slice<'a>) -> bool {
        self.find_token( input.current )
    }
}

impl<'a,'b> FindToken<Slice<'a>> for &'b u8 {
    fn find_token(&self, input: Slice<'a>) -> bool {
        self.find_token( input.current )
    }
}

impl<'a> FindToken<Slice<'a>> for char {
    fn find_token(&self, input: Slice<'a>) -> bool {
        self.find_token( input.current )
    }
}

impl<'a,'b> FindSubstring<Slice<'b>> for &'a [u8] {
    fn find_substring(&self, substr: Slice<'b>) -> Option<usize> {
        self.find_substring( substr.current )
    }
}

impl<'a,'b> FindSubstring<Slice<'b>> for &'a str {
    //returns byte index
    fn find_substring(&self, substr: Slice<'b>) -> Option<usize> {
        self.find_substring( substr.current )
    }
}

impl<'a,'b> FindSubstring<Slice<'b>> for Slice<'a> {
    //returns byte index
    fn find_substring(&self, substr: Slice<'b>) -> Option<usize> {
        self.current.find_substring( substr.current )
    }
}

impl<'a,'b> FindSubstring<&'b str> for Slice<'a> {
    //returns byte index
    fn find_substring(&self, substr: &'b str) -> Option<usize> {
        self.current.find_substring( substr )
    }
}

macro_rules! impl_slice_start {
    ($($kind:ty),*) => { $(
        impl<'a> NomSlice<$kind> for Slice<'a> {
            fn slice( &self, range: $kind) -> Self {
                let base_offset = range.start;
                Slice {
                    base_offset,
                    current: &self.current[range],
                }
            }
        }
    )* }
}

impl_slice_start!( Range<usize>, RangeFrom<usize> );

macro_rules! impl_slice_id {
    ($($kind:ty),*) => { $(
        impl<'a> NomSlice<$kind> for Slice<'a> {
            fn slice( &self, range: $kind) -> Self {
                Slice {
                    base_offset: self.base_offset,
                    current: &self.current[range],
                }
            }
        }
    )* }
}

impl_slice_id!( RangeTo<usize>, RangeFull );
