

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

#[macro_export]
macro_rules! deref0 {
    ($name:ident => $tp:ty) => (
        impl Deref for $name {
            type Target = $tp;
            fn deref( &self ) -> &Self::Target {
                &self.0
            }
        }
    );
    (+mut $name:ident => $tp:ty) => (
        deref0!{ $name => $tp }
        impl DerefMut for $name {
            fn deref_mut( &mut self ) -> &mut Self::Target {
                &mut self.0
            }
        }
    );
}