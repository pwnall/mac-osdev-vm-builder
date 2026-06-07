use anyhow::Result;
use std::path::Path;

#[allow(async_fn_in_trait)]
pub trait Stage: Send + Sync {
    fn name(&self) -> &'static str;

    async fn is_completed(
        &self,
        state_dir: &Path,
        effects: &dyn crate::effects::Effects,
    ) -> Result<bool> {
        let marker = state_dir.join(format!("{}.done", self.name()));
        Ok(effects.fs_exists(&marker).await)
    }

    async fn execute(&self, state_dir: &Path, effects: &dyn crate::effects::Effects) -> Result<()>;

    async fn mark_completed(
        &self,
        state_dir: &Path,
        effects: &dyn crate::effects::Effects,
    ) -> Result<()> {
        let marker = state_dir.join(format!("{}.done", self.name()));
        effects.fs_write(&marker, b"").await?;
        Ok(())
    }

    async fn reset(&self, state_dir: &Path, effects: &dyn crate::effects::Effects) -> Result<()> {
        let marker = state_dir.join(format!("{}.done", self.name()));
        if effects.fs_exists(&marker).await {
            effects.fs_remove_file(&marker).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::MockEffects;
    use anyhow::Result;
    use std::path::Path;

    struct DummyStage;
    impl Stage for DummyStage {
        fn name(&self) -> &'static str {
            "dummy"
        }
        async fn execute(
            &self,
            _state_dir: &Path,
            _effects: &dyn crate::effects::Effects,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_stage_completion() -> Result<()> {
        let mock = MockEffects::new();
        let stage = DummyStage;
        let state_dir = Path::new("/state");

        // Initially not completed
        assert!(!stage.is_completed(state_dir, &mock).await?);

        // Mark completed
        stage.mark_completed(state_dir, &mock).await?;
        assert!(stage.is_completed(state_dir, &mock).await?);

        // Reset
        stage.reset(state_dir, &mock).await?;
        assert!(!stage.is_completed(state_dir, &mock).await?);

        Ok(())
    }
}
