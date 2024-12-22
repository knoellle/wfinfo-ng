use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum BestItemMode {
    #[default]
    Default,
    Platinum,
    Ducats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum InfoMode {
    #[default]
    Default,
    All,
}
