use crate::dependency::ManifestDependency;
use crate::registry::ReleaseFetchProgress;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use std::time::Duration;

pub(crate) struct ReleaseFetchProgressBar {
    bar: ProgressBar,
}

impl ReleaseFetchProgressBar {
    pub(crate) fn new(total: usize) -> Self {
        if total == 0 {
            return Self {
                bar: ProgressBar::hidden(),
            };
        }

        let bar = ProgressBar::new(total as u64);
        bar.set_style(Self::style());
        bar.set_message("starting");
        bar.enable_steady_tick(Duration::from_millis(100));

        Self { bar }
    }

    pub(crate) fn finish(self) {
        self.bar.finish_and_clear();
    }

    fn style() -> ProgressStyle {
        ProgressStyle::with_template(
            "{spinner:.green} querying crates.io [{bar:32.cyan/blue}] {pos}/{len} {wide_msg}",
        )
        .expect("release fetch progress template should be valid")
        .progress_chars("=>-")
    }
}

impl ReleaseFetchProgress for ReleaseFetchProgressBar {
    fn dependency_finished(&self, dependency: &ManifestDependency) {
        self.bar.inc(1);
        self.bar.set_message(format!("fetched {}", dependency.name));
    }
}
