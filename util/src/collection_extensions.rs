
/// Extension traits for collections in Rust
/// These provide similar functionality to the Kotlin extension functions

/// Trait for weighted random selection from a list
pub trait RandomWeighted<T> {
    /// Get one random element of a given list.
    ///
    /// The probability for each element is proportional to the value of its corresponding element in the weights list.
    fn random_weighted(&self, weights: &[f32], rng: &mut impl Rng) -> Result<&T, String>;

    /// Get one random element of a given list.
    ///
    /// The probability for each element is proportional to the result of get_weight (evaluated only once).
    fn random_weighted_by<F>(&self, rng: &mut impl Rng, get_weight: F) -> Result<&T, String>
    where
        F: Fn(&T) -> f32;
}

impl<T> RandomWeighted<T> for [T] {
    fn random_weighted(&self, weights: &[f32], rng: &mut impl Rng) -> Result<&T, String> {
        if self.is_empty() {
            return Err("Empty list.".to_string());
        }
        if self.len() != weights.len() {
            return Err("Weights size does not match this list size.".to_string());
        }

        let total_weight: f32 = weights.iter().sum();
        let rand_double = rng.gen::<f32>();
        let mut sum = 0.0;

        for i in 0..weights.len() {
            sum += weights[i] / total_weight;
            if rand_double <= sum {
                return Ok(&self[i]);
            }
        }
        Ok(&self[self.len() - 1])
    }

    fn random_weighted_by<F>(&self, rng: &mut impl Rng, get_weight: F) -> Result<&T, String>
    where
        F: Fn(&T) -> f32,
    {
        let weights: Vec<f32> = self.iter().map(|item| get_weight(item)).collect();
        self.random_weighted(&weights, rng)
    }
}

/// Trait for adding an item to a collection
pub trait WithItem<T> {
    /// Gets a clone of a collection with an additional item
    ///
    /// Solves concurrent modification problems - everyone who had a reference to the previous collection can keep using it because it hasn't changed
    fn with_item(&self, item: T) -> Self where Self: Sized;
}

impl<T: Clone> WithItem<T> for Vec<T> {
    fn with_item(&self, item: T) -> Self {
        let mut new_vec = self.clone();
        new_vec.push(item);
        new_vec
    }
}

impl<T: Clone + std::hash::Hash + std::cmp::Eq> WithItem<T> for HashSet<T> {
    fn with_item(&self, item: T) -> Self {
        let mut new_set = self.clone();
        new_set.insert(item);
        new_set
    }
}

/// Trait for removing an item from a collection
pub trait WithoutItem<T> {
    /// Gets a clone of a collection without a given item
    ///
    /// Solves concurrent modification problems - everyone who had a reference to the previous collection can keep using it because it hasn't changed
    fn without_item(&self, item: &T) -> Self where Self: Sized;
}

impl<T: Clone + PartialEq> WithoutItem<T> for Vec<T> {
    fn without_item(&self, item: &T) -> Self {
        let mut new_vec = self.clone();
        new_vec.retain(|x| x != item);
        new_vec
    }
}

impl<T: Clone + std::hash::Hash + std::cmp::Eq> WithoutItem<T> for HashSet<T> {
    fn without_item(&self, item: &T) -> Self {
        let mut new_set = self.clone();
        new_set.remove(item);
        new_set
    }
}

/// Trait for converting to a GdxArray
///
/// Note: In Rust, we'll use a Vec instead of GdxArray
pub trait ToGdxArray<T> {
    /// Converts an iterable to a GdxArray (Vec in Rust)
    fn to_gdx_array(&self) -> Vec<T> where T: Clone;
}

impl<T: Clone> ToGdxArray<T> for Vec<T> {
    fn to_gdx_array(&self) -> Vec<T> {
        self.clone()
    }
}

impl<T: Clone> ToGdxArray<T> for &[T] {
    fn to_gdx_array(&self) -> Vec<T> {
        self.to_vec()
    }
}

/// Trait for yielding items from a sequence
///
/// Note: In Rust, we'll use a different approach since we don't have coroutines
pub trait YieldIfNotNull<T> {
    /// Yields an element if it's not null
    fn yield_if_not_null(&self, element: Option<&T>, f: &mut impl FnMut(&T));

    /// Yields all elements if they're not null
    fn yield_all_not_null<I>(&self, elements: Option<I>, f: &mut impl FnMut(&T))
    where
        I: IntoIterator<Item = T>,
        T: Clone;

    /// Yields all non-null elements if the collection is not null
    fn yield_all_not_null_null<I>(&self, elements: Option<I>, f: &mut impl FnMut(&T))
    where
        I: IntoIterator<Item = Option<T>>,
        T: Clone;
}

impl<T> YieldIfNotNull<T> for () {
    fn yield_if_not_null(&self, element: Option<&T>, f: &mut impl FnMut(&T)) {
        if let Some(e) = element {
            f(e);
        }
    }

    fn yield_all_not_null<I>(&self, elements: Option<I>, f: &mut impl FnMut(&T))
    where
        I: IntoIterator<Item = T>,
        T: Clone,
    {
        if let Some(elements) = elements {
            for element in elements {
                f(&element);
            }
        }
    }

    fn yield_all_not_null_null<I>(&self, elements: Option<I>, f: &mut impl FnMut(&T))
    where
        I: IntoIterator<Item = Option<T>>,
        T: Clone,
    {
        if let Some(elements) = elements {
            for element in elements {
                if let Some(e) = element {
                    f(&e);
                }
            }
        }
    }
}

/// Trait for adding to a map of sets
pub trait AddToMapOfSets<K, V> {
    /// Simplifies adding to a map of sets where the map entry where the new element belongs is not
    /// guaranteed to be already present in the map (sparse map).
    ///
    /// Returns `false` if the element was already present, `true` if it was new (same as `Set.add()`)
    fn add_to_map_of_sets(&mut self, key: K, element: V) -> bool
    where
        K: std::hash::Hash + std::cmp::Eq,
        V: std::hash::Hash + std::cmp::Eq;
}

impl<K, V> AddToMapOfSets<K, V> for HashMap<K, HashSet<V>> {
    fn add_to_map_of_sets(&mut self, key: K, element: V) -> bool
    where
        K: std::hash::Hash + std::cmp::Eq,
        V: std::hash::Hash + std::cmp::Eq,
    {
        self.entry(key).or_insert_with(HashSet::new).insert(element)
    }
}

/// Trait for checking if a map of sets contains an element
pub trait ContainsInMapOfSets<K, V> {
    /// Simplifies testing whether in a sparse map of sets the element exists for key.
    fn contains_in_map_of_sets(&self, key: &K, element: &V) -> bool
    where
        K: std::hash::Hash + std::cmp::Eq,
        V: std::hash::Hash + std::cmp::Eq;
}

impl<K, V> ContainsInMapOfSets<K, V> for HashMap<K, HashSet<V>> {
    fn contains_in_map_of_sets(&self, key: &K, element: &V) -> bool
    where
        K: std::hash::Hash + std::cmp::Eq,
        V: std::hash::Hash + std::cmp::Eq,
    {
        self.get(key).map_or(false, |set| set.contains(element))
    }
}

/// Extension trait for HashMap to provide get_or_put functionality
pub trait GetOrPut<K, V> {
    /// Gets the value for the given key or inserts a new value if the key doesn't exist
    fn get_or_put<F>(&mut self, key: K, default: F) -> &mut V
    where
        K: std::hash::Hash + std::cmp::Eq,
        F: FnOnce() -> V;
}

impl<K, V> GetOrPut<K, V> for HashMap<K, V> {
    fn get_or_put<F>(&mut self, key: K, default: F) -> &mut V
    where
        K: std::hash::Hash + std::cmp::Eq,
        F: FnOnce() -> V,
    {
        if !self.contains_key(&key) {
            self.insert(key.clone(), default());
        }
        self.get_mut(&key).unwrap()
    }
}