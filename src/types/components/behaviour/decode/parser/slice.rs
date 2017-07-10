use std::ops::{ Range, RangeFrom, RangeTo, RangeFull };
use nom;
use super::*;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
struct Slice<'a> {
    current: &'a [str],
    base_offset: usize
}

impl Slice {
    fn new<'a>( base: &'a [str] ) -> Slice<'a> {
        Slice {
            current: base,
            base_offset: 0
        }
    }

    fn as_base_range( &self ) -> Range<usize> {
        Range { start: self.base_offset, end: self.base_offset + self.current.len() }
    }
}




impl<'a> nom::InputIter for Slice<'a> {
    type Item = <&'a str as nom::InputIter>::Item;
    type RawItem = <&'a str as nom::InputIter>::RawItem;
    type Iter = <&'a str as nom::InputIter>::Iter;
    type IterElem = <&'a str as nom::InputIter>::IterElem;

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

impl<'a> nom::InputLength for Slice<'a> {
    #[inline(always)]
    fn input_len(&self) -> usize {
        self.current.input_len()
    }
}

impl<'a> nom::InputTake for Slice<'a> {
    #[inline(always)]
    fn take<P>(&self, count: usize)  -> Option<&Self> {
        self.current.take( count ).map( |strslice| {
            Slice {
                current: strslice,
                base_offset: self.base_offset
            }
        })
    }
    #[inline(always)]
    fn take_split<P>(&self, count: usize) -> Option<(&Self,&Self)> {
        self.current.take_split().map( |(from_count, until_count)| {
            ( Slice { current: from_count, base_offset: self.base_offset + count },
              Slice { current: until_count, base_offset: self.base_offset } )
        })
    }
}

impl<'a, T> nom::Compare<T> for Slice<'a> where &'a str: Compare<T> {

    fn compare(&self, t: T) -> CompareResult {
        self.current.compare( t )
    }

    fn compare_no_case(&self, t: T) -> CompareResult {
        self.current.compare_no_case( t )
    }
}

impl<'a> nom::FindToken<Slice<'a>> for u8 {
    fn find_token(&self, input: Slice<'a>) -> bool {
        self.find_token( input.current )
    }
}

impl<'a,'b> nom::FindToken<Slice<'a>> for &'b u8 {
    fn find_token(&self, input: Slice<'a>) -> bool {
        self.find_token( input.current )
    }
}

impl<'a> nom::FindToken<Slice<'a>> for char {
    fn find_token(&self, input: &str) -> bool {
        self.find_token( input.current )
    }
}

impl<'a,'b> nom::FindSubstring<Slice<'b>> for &'a [u8] {
    fn find_substring(&self, substr: Slice<'b>) -> Option<usize> {
        self.find_substring( substr.current )
    }
}

impl<'a,'b> nom::FindSubstring<Slice<'b>> for &'a str {
    //returns byte index
    fn find_substring(&self, substr: Slice<'b>) -> Option<usize> {
        self.find_substring( substr.current )
    }
}

impl<'a,'b> nom::FindSubstring<Slice<'b>> for Slice<'a> {
    //returns byte index
    fn find_substring(&self, substr: Slice<'b>) -> Option<usize> {
        self.current.find_substring( substr.current )
    }
}

impl<'a,'b> nom::FindSubstring<&'b str> for Slice<'a> {
    //returns byte index
    fn find_substring(&self, substr: &'b str) -> Option<usize> {
        self.current.find_substring( substr )
    }
}

macro_rules! impl_slice_start {
    ($($kind:ty),*) => { $(
        impl<'a> nom::Slice<$kind> for Slice<'a> {
            fn slice( &self, range: $kind) -> Self {
                let base_offset = range.start;
                Slice {
                    base_offset,
                    current: self.current[range],
                }
            }
        }
    )* }
}

impl_slice_start!( Range<usize>, RangeFrom<usize> );

macro_rules! impl_slice_id {
    ($($kind:ty),*) => { $(
        impl<'a> nom::Slice<$kind> for Slice<'a> {
            fn slice( &self, range: $kind) -> Self {
                Slice {
                    base_offset: self.base_offset,
                    current: self.current[range],
                }
            }
        }
    )* }
}

impl_slice_id!( RangeTo<usize>, RangeFull );
