//! Where the links of the pages lead: the addresses of this site for [`Jump`].

use crate::view::Jump;
use crate::web::connections::connection_query;
use crate::web::entries::search_query;
use crate::web::packets::filters_href;

/// The address of a link; templates call `{{ r.jump.href() }}`
pub trait Href {
    fn href(&self) -> String;
}

impl Href for Jump {
    fn href(&self) -> String {
        match self {
            Jump::Connection(key) => format!("/connection?{}", connection_query(key)),
            Jump::Connections { filter } if filter.is_empty() => "/connections".to_owned(),
            Jump::Connections { filter } => format!("/connections?{}", search_query(filter)),
            Jump::Packets {
                filter,
                connections,
            } => filters_href("/packets", filter, connections),
            Jump::Statistics {
                filter,
                connections,
            } => filters_href("/statistics", filter, connections),
            Jump::Stream(number) => format!("/stream?n={number}"),
        }
    }
}
