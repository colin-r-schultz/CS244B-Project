use crate::types::ResourceConfig;

pub trait Resources: Send + Sync {
    fn from_config(config: &ResourceConfig) -> Self;

    fn can_execute<'a>(
        mut reqs: impl Iterator<Item = (&'a str, usize)>,
        resources: &ResourceConfig,
    ) -> bool {
        reqs.all(|(res, amt)| resources.get(res).map(|&cap| amt <= cap).unwrap_or(false))
    }
}

impl Resources for () {
    fn from_config(_config: &ResourceConfig) -> Self {}
}

pub struct CpuResources {
    count: usize,
}

impl Resources for CpuResources {
    fn from_config(config: &ResourceConfig) -> Self {
        Self {
            count: *config.get("cpus").unwrap_or(&0),
        }
    }
}

impl CpuResources {
    pub fn cpus<const N: usize>(&self) -> [(); N] {
        [(); N]
    }
}

pub struct CpuHandle;
