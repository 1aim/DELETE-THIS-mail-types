

#[macro_export]
macro_rules! ascii_str {
    ($($ch:ident)*) => {{
        use $crate::ascii::{ AsciiStr, AsciiChar };
        type RA = &'static AsciiStr;
        static STR: &[AsciiChar] = &[ $(AsciiChar::$ch),* ];
        RA::from( STR )
    }}
}

#[macro_export]
macro_rules! sep_for {
    ($var:ident in $iter:expr; sep $sep:block; $($rem:tt)* ) => {{
        let mut first = true;
        for $var in $iter {
            if first { first = false; }
            else {
                $sep
            }
            $( $rem )*
        }
    }}
}