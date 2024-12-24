use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum BestItemMode {
    #[default]
    Combined,
    Platinum,
    Ducats,
    Volatility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum InfoDisplayMode {
    #[default]
    Minimal,
    Combined,
    All,
}
