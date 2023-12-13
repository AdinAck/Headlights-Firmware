#[macro_export]
macro_rules! assign_resources {
    {
        $(
            $group_name:ident : $group_struct:ident {
                $($resource_name:ident : $resource_field:ident $(=$ALIAS:ident)?),*
                $(,)?
            }
            $(,)?
        )+
    } => {
        $($($(type $ALIAS = $resource_field;)?)*)*

        #[allow(dead_code,non_snake_case)]
        struct AssignedResources {
            $($group_name : $group_struct),*
        }
        $(
            #[allow(dead_code,non_snake_case)]
            struct $group_struct {
                $(pub $resource_name: peripherals::$resource_field),*
            }
        )+
        macro_rules! split_resources (
            ($p:ident) => {
                AssignedResources {
                    $($group_name: $group_struct {
                        $($resource_name: $p.$resource_field),*
                    }),*
                }
            }
        );
    };
    {
        $(
            $pub:vis $group_name:ident : $group_struct:ident {
                $($resource_name:ident : $resource_field:ident $(=$ALIAS:ident)?),*
                $(,)?
            }
            $(,)?
        )+
    } => {
        $($($($pub type $ALIAS = $resource_field;)?)*)*

        #[allow(dead_code,non_snake_case)]
        struct AssignedResources {
            $($group_name : $group_struct),*
        }
        $(
            #[allow(dead_code,non_snake_case)]
            $pub struct $group_struct {
                $(pub $resource_name: peripherals::$resource_field),*
            }
        )+
        macro_rules! split_resources (
            ($p:ident) => {
                AssignedResources {
                    $($group_name: $group_struct {
                        $($resource_name: $p.$resource_field),*
                    }),*
                }
            }
        );
    }
}
