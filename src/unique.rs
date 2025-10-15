pub trait FromUniqueIterator<A>: Sized {
    fn from_unique_iter<T: IntoIterator<Item = A>>(iter: T) -> Self;
}

pub trait UniqueIterator: Iterator {
    fn collect_unique<B: FromUniqueIterator<Self::Item>>(self) -> B
    where
        Self: Sized,
    {
        FromUniqueIterator::from_unique_iter(self)
    }
}

impl<I: Iterator> UniqueIterator for I {}

impl<A: PartialEq> FromUniqueIterator<A> for Vec<A> {
    fn from_unique_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let mut vec = Vec::new();
        iter.into_iter().for_each(|i| {
            if !vec.contains(&i) {
                vec.push(i)
            }
        });
        vec
    }
}
