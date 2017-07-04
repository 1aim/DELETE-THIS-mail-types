

#[macro_export]
macro_rules! ascii_str {
    ($($ch:ident)*) => {{
        use $crate::ascii::{ AsciiStr, AsciiChar };
        type RA = &'static AsciiStr;
        static STR: &[AsciiChar] = &[ $(AsciiChar::$ch),* ];
        RA::from( STR )
    }}
}