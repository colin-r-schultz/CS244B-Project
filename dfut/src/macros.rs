#[macro_export]
macro_rules! create_struct {
    ($name:ident ($($arg:ident : $argtype:ty),*) $ret:ty $body:block) => {
        #[allow(non_camel_case_types)]
        #[derive($crate::macros::support::Serialize,$crate::macros::support::Deserialize)]
        pub struct $name<$(#[allow(non_camel_case_types)] $arg = $crate::macros::support::MaybeFut<$argtype>),*>($($arg,)*);

        #[allow(non_camel_case_types)]
        impl<$($arg),*> $name<$($arg),*> {
            fn _run($($arg : $argtype),*) -> impl std::future::Future<Output = $ret> + Send + 'static { async move { $body } }
        }

        #[allow(non_camel_case_types)]
        impl<$($arg: $crate::macros::support::MaybeFutTrait<$argtype>),*> Into<dfut_impl::Call> for $name<$($arg),*> {
            fn into(self) -> dfut_impl::Call {
                let Self($($arg),*) = self;
                dfut_impl::Call::$name($name($($arg.into()),*))
            }
        }

        #[allow(non_camel_case_types)]
        impl<$($arg: $crate::macros::support::MaybeFutTrait<$argtype>),*> $crate::macros::support::DFutCall<dfut_impl::Call> for $name<$($arg),*> {
            type Output = $ret;

            fn run(self, node: &'static $crate::Node<dfut_impl::Call>) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
                let Self($($arg),*) = self;
                async move {
                    Self::_run($($arg.retrieve(node).await),*).await
                }
            }

            fn get_dfut_deps(&self) -> impl Iterator<Item = ($crate::macros::support::NodeId, $crate::macros::support::DFutId)> {
                let res = None.into_iter();
                let Self($($arg),*) = self;
                $(let res = res.chain($arg.get_remote_dep());)*
                res
            }
        }

        #[allow(non_camel_case_types)]
        impl<$($arg: $crate::macros::support::Resolve<$argtype>),*> std::future::IntoFuture for $name<$($arg),*> {
            type Output = $ret;
            type IntoFuture = std::pin::Pin<std::boxed::Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

            fn into_future(self) -> Self::IntoFuture {
                let Self($($arg),*) = self;
                std::boxed::Box::pin(
                    async {
                        Self::_run($($arg.resolve().await),*).await
                    })
            }
        }
    };
}

#[macro_export]
macro_rules! dfut_procs {
    ($(async fn $name:ident ($($arg:ident : $argtype:ty),*) -> $ret:ty $body:block)*) => {
        #[allow(non_camel_case_types)]
        mod dfut_impl {
            use std::sync::Arc;
            use $crate::macros::support::{DFutTrait, DFutCall, Serialize, Deserialize, Node, Value, DFutId, NodeId,};

            #[derive(Serialize,Deserialize)]
            pub enum Call {
                $($name(super::$name)),*
            }

            impl DFutTrait for Call{}

            impl DFutCall<Self> for Call {
                type Output = Value;

                async fn run(self, node: &'static Node<Self>) -> Self::Output {
                    match self {
                        $(Self::$name(inner) => Arc::new(inner.run(node).await)),*
                    }
                }

                fn get_dfut_deps(&self) -> impl Iterator<Item = (NodeId, DFutId)> {
                    let vec: Vec<_> = match self {
                        $(Self::$name(inner) => inner.get_dfut_deps().collect()),*
                    };
                    vec.into_iter()
                }
            }
        }
        $($crate::create_struct!{$name ($($arg : $argtype),*) $ret $body})*

    };
}

// dfut_procs! {
//     async fn add(a: i32, b: i32) -> i32 {
//         a + b
//     }
// }

pub mod support {
    pub use crate::dfut::{DFutCall, DFutTrait, MaybeFut, MaybeFutTrait, Resolve};
    pub use crate::node::Node;
    pub use crate::types::{DFutId, NodeId, Value};
    pub use serde::{Deserialize, Serialize};
}
