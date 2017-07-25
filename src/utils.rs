

//trait PushIfSome<T> {
//    fn push_if_some( &mut self, val: Option<T> );
//}
//
//impl<T> PushIfSome<T> for Vec<T> {
//    #[inline]
//    fn push_if_some( &mut self, val: Option<T> ) {
//        if let Some( val ) = val {
//            self.push( val );
//        }
//    }
//}


//modified, origin is:
// https://github.com/rust-lang/rust/blob/2fbba5bdbadeef403a64e9e1568cdad225cbcec1/src/liballoc/string.rs
pub fn insert_bytes(vec: &mut Vec<u8> , idx: usize, bytes: &[u8]) {
    use std::ptr;
    let len = vec.len();
    let amount = bytes.len();
    vec.reserve(amount);

    unsafe  {
        ptr::copy( vec.as_ptr().offset( idx as isize ),
                   vec.as_mut_ptr().offset( (idx + amount) as isize ),
                   len - idx );
        ptr::copy( bytes.as_ptr(),
                   vec.as_mut_ptr().offset( idx as isize ),
                   amount );

        vec.set_len( len + amount );
    }
}





