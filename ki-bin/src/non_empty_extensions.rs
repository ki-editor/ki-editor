use itertools::Itertools;
use nonempty::NonEmpty;

pub trait NonEmptyTryCollectResult<T> {
    fn try_collect(self) -> anyhow::Result<NonEmpty<T>>;
}

pub trait NonEmptyTryCollectOption<T> {
    fn try_collect(self) -> Option<NonEmpty<T>>;
}

impl<T> NonEmptyTryCollectResult<T> for NonEmpty<anyhow::Result<T>> {
    fn try_collect(self) -> anyhow::Result<NonEmpty<T>> {
        let head = self.head?;
        let tail = self.tail.into_iter().try_collect()?;
        Ok(NonEmpty { head, tail })
    }
}

impl<T> NonEmptyTryCollectOption<T> for NonEmpty<Option<T>> {
    fn try_collect(self) -> Option<NonEmpty<T>> {
        let head = self.head?;
        let tail = self.tail.into_iter().collect::<Option<Vec<_>>>()?;
        Some(NonEmpty { head, tail })
    }
}
