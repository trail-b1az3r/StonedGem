//! Content blocking for StonedGem browsers, independent of the engine:
//!
//! * [`blocker`]: Adblock/uBlock filter matching for requests and element
//!   hiding, with statistics. The engine asks [`blocker::should_block`]
//!   for every request.
//! * [`filterlists`]: the catalogue of subscribable lists and their files.
//! * [`netrules`]: rules power extensions add at runtime from `network.ts`
//!   (blocker rules, request headers, the request feed).
//!
//! The product sets the profile folders (`stonedgem-profile`) and the
//! extension filter files ([`blocker::set_extension_filters`]).

pub mod blocker;
pub mod filterlists;
pub mod netrules;
