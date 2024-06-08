#[macro_export]
macro_rules! create_struct {
    ([$($resource:ident $amt:literal $alias:ident $($_:ident)?),*] $name:ident ($($arg:ident : $argtype:ty),*) $ret:ty $body:block) => {
        #[allow(non_camel_case_types)]
        #[derive($crate::macros::support::Serialize,$crate::macros::support::Deserialize)]
        pub struct $name<$(#[allow(non_camel_case_types)] $arg = $crate::macros::support::MaybeFut<$argtype>),*>($($arg,)*);

        // #[allow(non_camel_case_types)]
        // impl<$($arg),*> $name<$($arg),*> {
        //     fn _run($($arg : $argtype),*) -> impl std::future::Future<Output = $ret> + Send + 'static { async move { $body } }
        // }

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
                $(let $alias = node.resources().$resource::<$amt>();)*
                async move {
                    (|$($arg : $argtype,)*| async move $body
                )($($arg.retrieve(node).await,)*).await
                }
            }

            fn get_dfut_deps(&self) -> impl Iterator<Item = ($crate::macros::support::NodeId, $crate::macros::support::DFutId)> {
                let res = None.into_iter();
                let Self($($arg),*) = self;
                $(let res = res.chain($arg.get_remote_dep());)*
                res
            }

            fn get_resource_deps(&self) -> impl Iterator<Item = (&str, usize)> {
                [
                    $((stringify!($resource), $amt)),*
                ].into_iter()
            }
        }

        // #[allow(non_camel_case_types)]
        // impl<$($arg: $crate::macros::support::Resolve<$argtype>),*> std::future::IntoFuture for $name<$($arg),*> {
        //     type Output = $ret;
        //     type IntoFuture = std::pin::Pin<std::boxed::Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

        //     fn into_future(self) -> Self::IntoFuture {
        //         let Self($($arg),*) = self;
        //         std::boxed::Box::pin(
        //             async {
        //                 Self::_run($($arg.resolve().await),*).await
        //             })
        //     }
        // }
    };
}

#[macro_export]
macro_rules! or_else {
    ($x:tt $($_:tt)?) => {
        $x
    };
}

#[macro_export]
macro_rules! dfut_procs {
    ($(#![resources( $resources:ty )])?

     $( $(#[requires( $($res:ident($amt:literal) $(as $alias:ident)?),+ )])?
        async fn $name:ident ($($arg:ident : $argtype:ty),*) -> $ret:ty $body:block)*) => {
        #[allow(non_camel_case_types)]
        mod dfut_impl {
            use std::sync::Arc;
            use $crate::macros::support::{DFutCall, Serialize, Deserialize, Node, Value, DFutId, NodeId,};

            #[derive(Serialize,Deserialize)]
            pub enum Call {
                $($name(super::$name)),*
            }

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

                fn get_resource_deps(&self) -> impl Iterator<Item = (&str, usize)> {
                    let vec: Vec<_> = match self {
                        $(Self::$name(inner) => inner.get_resource_deps().collect()),*
                    };
                    vec.into_iter()
                }
            }
        }

        impl $crate::macros::support::DFutTrait for dfut_impl::Call{
            type Resources = $crate::or_else!{$($resources)? ()};
        }

        $($crate::create_struct!{
            [$($($res $amt $($alias)? $res),*)?]
            $name ($($arg : $argtype),*) $ret $body
        })*

    };
}

// dfut_procs! {
//     #[resources(crate::resource::CpuResources)]

//     #[requires(cpus(3))]
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
