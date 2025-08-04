#[macro_export]
macro_rules! make_getters {

    // Return by reference
    (ref: $( ($field:ident : $type:ty) ),* $(,)?) => {
        $(
            pub fn $field(&self) -> &$type {
                &self.$field
            }
        )*
    };

    // Return by copy
    (copy: $( ($field: ident : $type:ty) ),* $(,)?) => {
        $(
            pub fn $field(&self) -> $type {
                self.$field
            }
        )*
    };
}

