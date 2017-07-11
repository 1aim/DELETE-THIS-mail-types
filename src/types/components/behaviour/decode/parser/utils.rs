
// Note:
// macros in this module are bad at imports, they are only meant to
// be used in the crate, if you export them imports may die
// (oh and you have to have types::components::behaviour::decoder::parser::slice::Slice in scope)
//

#[macro_export]
macro_rules! my_named {
    ($name:ident, $submac:ident!( $($args:tt)* )) => (
        my_named!($name< Slice<'a> >, $submac!( $($args)* ) );
    );
    ($name:ident<$o:ty>, $submac:ident!( $($args:tt)* )) => (
        //FIXME replace u32 with custom error e.g. quick_error{...}
        #[allow(unused_variables)]
        pub fn $name<'a>( input: Slice<'a> ) -> ::nom::IResult<Slice<'a>, $o, u32> {
            $submac!( input, $($args)* )
        }
    );
}

#[macro_export]
macro_rules! verify_char (
  ($i:expr, $c: expr) => (
    {
      //Note: internal use only, no need to reexport nom + use $crate::...
      use nom::{ IResult, Needed, ErrorKind, Slice, AsChar, InputIter };

      match ($i).iter_elements().next().map(|c| {
        let func = $c;
        func( c.as_char() )
      }) {
        None        => IResult::Incomplete::<_, _>( Needed::Size( 1 ) ),
        Some(false) => IResult::Error( error_position!( ErrorKind::Verify, $i ) ),
        Some(true)  => IResult::Done(
            $i.slice( 1.. ),
            $i.iter_elements().next().unwrap().as_char())
      }
    }
  );
);

#[macro_export]
macro_rules! void {
    () => { |_|() }
}

#[macro_export]
macro_rules! postceded(
    ($i:expr, $submac:ident!( $($args:tt)* ), $submac2:ident!( $($args2:tt)* )) => {{
        use ::nom::IResult::*;
        match tuple!($i, $submac!($($args)*), $submac2!($($args2)*)) {
            Error( err ) => Error( err),
            Incomplete( need ) => Incomplete( need ),
            Done( remaining, ( out, _ ) )    => Done( remaining, out )
        }
    }};

    ($i:expr, $submac:ident!( $($args:tt)* ), $g:expr) => {
        postceded!($i, $submac!($($args)*), call!($g));
    };

    ($i:expr, $f:expr, $submac:ident!( $($args:tt)* )) => {
        postceded!($i, call!($f), $submac!($($args)*));
    };

    ($i:expr, $f:expr, $g:expr) => {
        postceded!($i, call!($f), call!($g));
    };
);

