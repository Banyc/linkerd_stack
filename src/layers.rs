use tower::layer::util::Identity;

/// The same as `tower#ServiceBuilder` but with a upside-down execution order.
///
/// The execution order is in line with `Stack`.
///
/// The execution order is from the bottom to the top.
pub struct Layers<L>(L);

impl Layers<Identity> {
    pub fn new() -> Self {
        Layers(Identity::new())
    }
}

impl<L> Layers<L> {
    /// Push an outer layer onto the layer stack.
    pub fn push<O>(self, outer: O) -> Layers<tower::layer::util::Stack<L, O>> {
        Layers(tower::layer::util::Stack::new(self.0, outer))
    }
}
