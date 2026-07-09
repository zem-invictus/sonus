use crate::spatial_audio::filter::{FilterParams, FilterType};

#[derive(Debug, Clone, Copy)]
pub enum AudioCommand {
    UpdateFilter(FilterParams),
    EnableFilter(FilterType),
    DisableFilter(FilterType),
}