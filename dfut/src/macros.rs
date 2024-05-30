macro_rules! create_struct {
    ($name:ident ($($arg:ident : $argtype:ty),*) $ret:ty) => {
        #[derive(Serialize,Deserialize)]
        pub struct $name {
            $($arg : MaybeFut<$argtype>),*
        }

        impl $name {
            pub fn new($($arg : MaybeFut<$argtype>),*) -> Self {
                Self { $($arg),* }
            }
        }

        impl From<$name> for super::Call {
            fn from(val: $name) -> super::Call {
                super::Call::$name(val)
            }
        }

        impl DFutTrait for $name {
            type Output = $ret;
            type CallType = super::Call;

            async fn run(self, node: &'static Node<Self::CallType>) -> Self::Output {
                let Self { $($arg),* } = self;
                Self::_run($($arg.resolve(node).await),*).await
            }

            fn get_dfut_deps(&self) -> Vec<($crate::types::NodeId, $crate::types::DFutId)> {
                let mut res = Vec::new();
                $(self.$arg.add_dfut_dep(&mut res);)*
                res
            }
        }
    };
}

macro_rules! create_constructor {
    ($name:ident ($($arg:ident : $argtype:ty),*)) => {
        fn $name ($($arg : impl Into<$crate::dfut::MaybeFut<$argtype>>),*) -> dfut_impl::structs::$name {
            dfut_impl::structs::$name::new($($arg.into()),*)
        }
    };
}

macro_rules! struct_impl {
    ($name:ident ($($arg:ident : $argtype:ty),*) $ret:ty $body:block) => {
        impl dfut_impl::structs::$name {
            async fn _run($($arg : $argtype),*) -> $ret $body
        }
    };
}

#[macro_export]
macro_rules! dfut_procs {
    ($(async fn $name:ident ($($arg:ident : $argtype:ty),*) -> $ret:ty $body:block)*) => {
        #[allow(non_camel_case_types)]
        mod dfut_impl {
            use std::sync::Arc;

            use $crate::Node;
            use $crate::dfut::{DFutTrait, MaybeFut};
            use $crate::types::{Value};

            use serde::{Serialize, Deserialize};

            #[derive(Serialize,Deserialize)]
            pub enum Call {
                $($name(structs::$name)),*
            }

            pub(super) mod structs {
                use super::{DFutTrait, MaybeFut, Serialize, Deserialize, Node};
                $(create_struct!{$name ($($arg : $argtype),*) $ret})*
            }

            impl DFutTrait for Call {
                type Output = Value;
                type CallType = Self;

                async fn run(self, node: &'static Node<Self::CallType>) -> Self::Output {
                    match self {
                        $(Self::$name(inner) => Arc::new(inner.run(node).await)),*
                    }
                }

                fn get_dfut_deps(&self) -> Vec<($crate::types::NodeId, $crate::types::DFutId)> {
                    match self {
                        $(Self::$name(inner) => inner.get_dfut_deps()),*
                    }
                }
            }
        }

        $(create_constructor!{$name ($($arg : $argtype),*)})*
        $(struct_impl!{$name ($($arg : $argtype),*) $ret $body})*

    };
}

dfut_procs! {
    async fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    async fn wiki_ladder(_path: Vec<String>, _target: String) -> Vec<String> {
        vec!["a".to_owned()]
    }
}