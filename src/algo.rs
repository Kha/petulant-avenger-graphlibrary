//! Graph algorithms.
//!
//! It is a goal to gradually migrate the algorithms to be based on graph traits
//! so that they are generally applicable. For now, most of these use only the
//! **Graph** type.

use std::collections::BinaryHeap;
use std::borrow::Borrow;
use fb::FixedBitSet;

use super::{
    Graph,
    Directed,
    Undirected,
    EdgeType,
    Outgoing,
    Incoming,
    Dfs,
    MinScored,
};
use super::visit::{
    Reversed,
    Visitable,
    VisitMap,
};
use super::unionfind::UnionFind;
use super::graph::{
    NodeIndex,
    IndexType,
};

pub use super::isomorphism::is_isomorphic;
pub use super::dijkstra::dijkstra;

/// Return **true** if the input graph contains a cycle.
///
/// Treat the input graph as undirected.
pub fn is_cyclic_undirected<N, E, Ty, Ix>(g: &Graph<N, E, Ty, Ix>) -> bool where
    Ty: EdgeType,
    Ix: IndexType,
{
    let mut edge_sets = UnionFind::new(g.node_count());
    for edge in g.raw_edges() {
        let (a, b) = (edge.source(), edge.target());

        // union the two vertices of the edge
        //  -- if they were already the same, then we have a cycle
        if !edge_sets.union(a.index(), b.index()) {
            return true
        }
    }
    false
}

/// **Deprecated: Renamed to *is_cyclic_undirected*.**
pub fn is_cyclic<N, E, Ty, Ix>(g: &Graph<N, E, Ty, Ix>) -> bool where
    Ty: EdgeType,
    Ix: IndexType,
{
    is_cyclic_undirected(g)
}

pub struct Toposort<N, VM> {
    tovisit: Vec<N>,
    ordered: VM,
}

impl<Ix: IndexType> Toposort<NodeIndex<Ix>, FixedBitSet>
{
    /// Create a new **Toposort**, using the graph's visitor map, and put **start**
    /// in the stack of nodes to visit.
    pub fn new<N, E>(graph: &Graph<N, E, Directed, Ix>) -> Self
    {
        // find all initial nodes
        let tovisit = graph.without_edges(Incoming).collect();

        Toposort {
            ordered: graph.visit_map(),
            tovisit: tovisit,
        }
    }

    pub fn next<N, E>(&mut self, g: &Graph<N, E, Directed, Ix>) -> Option<NodeIndex<Ix>>
    {
        // Take an unvisited element and find which of its neighbors are next
        while let Some(nix) = self.tovisit.pop() {
            if self.ordered.is_visited(&nix) {
                continue;
            }
            self.ordered.visit(nix);
            for neigh in g.borrow().neighbors_directed(nix, Outgoing) {
                // Look at each neighbor, and those that only have incoming edges
                // from the already ordered list, they are the next to visit.
                if g.borrow().neighbors_directed(neigh, Incoming).all(|b| self.ordered.is_visited(&b)) {
                    self.tovisit.push(neigh);
                }
            }
            return Some(nix);
        }
        None
    }
}


/// Perform a topological sort of a directed graph **g**.
///
/// Visit each node in order (if it is part of a topological order).
///
/// You can pass **g** as either **&Graph** or **&mut Graph**, and it
/// will be passed on to the visitor closure.
#[inline]
pub fn toposort_generic<N, E, Ix, G, F>(mut g: G, mut visit: F)
    where Ix: IndexType,
          G: Borrow<Graph<N, E, Directed, Ix>>,
          F: FnMut(&mut G, NodeIndex<Ix>),
{
    let mut ordered = g.borrow().visit_map();
    let mut tovisit = Vec::new();

    // find all initial nodes
    tovisit.extend(g.borrow().without_edges(Incoming));

    // Take an unvisited element and find which of its neighbors are next
    while let Some(nix) = tovisit.pop() {
        if ordered.is_visited(&nix) {
            continue;
        }
        visit(&mut g, nix);
        ordered.visit(nix);
        for neigh in g.borrow().neighbors_directed(nix, Outgoing) {
            // Look at each neighbor, and those that only have incoming edges
            // from the already ordered list, they are the next to visit.
            if g.borrow().neighbors_directed(neigh, Incoming).all(|b| ordered.is_visited(&b)) {
                tovisit.push(neigh);
            }
        }
    }
}

/// Check if a directed graph contains cycles.
///
/// Using the topological sort algorithm.
///
/// Return **true** if the graph had a cycle.
pub fn is_cyclic_directed<N, E, Ix>(g: &Graph<N, E, Directed, Ix>) -> bool where
    Ix: IndexType,
{
    let mut n_ordered = 0;
    toposort_generic(g, |_, _| n_ordered += 1);
    n_ordered != g.node_count()
}

/// Perform a topological sort of a directed graph.
///
/// Return a vector of nodes in topological order: each node is ordered
/// before its successors.
///
/// If the returned vec contains less than all the nodes of the graph, then
/// the graph was cyclic.
pub fn toposort<N, E, Ix>(g: &Graph<N, E, Directed, Ix>) -> Vec<NodeIndex<Ix>> where
    Ix: IndexType,
{
    let mut order = Vec::with_capacity(g.node_count());
    toposort_generic(g, |_, ix| order.push(ix));
    order
}

/// Compute *Strongly connected components* using Kosaraju's algorithm.
///
/// Return a vector where each element is an scc.
///
/// For an undirected graph, the sccs are simply the connected components.
pub fn scc<N, E, Ty, Ix>(g: &Graph<N, E, Ty, Ix>) -> Vec<Vec<NodeIndex<Ix>>> where
    Ty: EdgeType,
    Ix: IndexType,
{
    let mut dfs = Dfs::empty(g);

    // First phase, reverse dfs pass, compute finishing times.
    let mut finish_order = Vec::new();
    for index in (0..g.node_count()) {
        if dfs.discovered.is_visited(&NodeIndex::<Ix>::new(index)) {
            continue
        }
        // We want to order the vertices by finishing time --
        // so record when we see them, then reverse all we have seen in this DFS pass.
        let pass_start = finish_order.len();
        dfs.move_to(NodeIndex::new(index));
        while let Some(nx) = dfs.next(&Reversed(g)) {
            finish_order.push(nx);
        }
        finish_order[pass_start..].reverse();
    }

    dfs.discovered.clear();
    let mut sccs = Vec::new();

    // Second phase
    // Process in decreasing finishing time order
    for &nindex in finish_order.iter().rev() {
        if dfs.discovered.is_visited(&nindex) {
            continue;
        }
        // Move to the leader node.
        dfs.move_to(nindex);
        //let leader = nindex;
        let mut scc = Vec::new();
        while let Some(nx) = dfs.next(g) {
            scc.push(nx);
        }
        sccs.push(scc);
    }
    sccs
}

/// Return the number of connected components of the graph.
///
/// For a directed graph, this is the *weakly* connected components.
pub fn connected_components<N, E, Ty, Ix>(g: &Graph<N, E, Ty, Ix>) -> usize where
    Ty: EdgeType,
    Ix: IndexType,
{
    let mut vertex_sets = UnionFind::new(g.node_count());
    for edge in g.raw_edges() {
        let (a, b) = (edge.source(), edge.target());

        // union the two vertices of the edge
        vertex_sets.union(a.index(), b.index());
    }
    let mut labels = vertex_sets.into_labeling();
    labels.sort();
    labels.dedup();
    labels.len()
}


/// Return a *Minimum Spanning Tree* of a graph.
///
/// Treat the input graph as undirected.
///
/// Using Kruskal's algorithm with runtime **O(|E| log |E|)**. We actually
/// return a minimum spanning forest, i.e. a minimum spanning tree for each connected
/// component of the graph.
///
/// The resulting graph has all the vertices of the input graph (with identical node indices),
/// and **|V| - c** edges, where **c** is the number of connected components in **g**.
pub fn min_spanning_tree<N, E, Ty, Ix>(g: &Graph<N, E, Ty, Ix>) -> Graph<N, E, Undirected, Ix> where
    N: Clone,
    E: Clone + PartialOrd,
    Ty: EdgeType,
    Ix: IndexType,
{
    if g.node_count() == 0 {
        return Graph::with_capacity(0, 0)
    }

    // Create a mst skeleton by copying all nodes
    let mut mst = Graph::with_capacity(g.node_count(), g.node_count() - 1);
    for node in g.raw_nodes() {
        mst.add_node(node.weight.clone());
    }

    // Initially each vertex is its own disjoint subgraph, track the connectedness
    // of the pre-MST with a union & find datastructure.
    let mut subgraphs = UnionFind::new(g.node_count());

    let mut sort_edges = BinaryHeap::with_capacity(g.edge_count());
    for edge in g.raw_edges() {
        sort_edges.push(MinScored(edge.weight.clone(), (edge.source(), edge.target())));
    }

    // Kruskal's algorithm.
    // Algorithm is this:
    //
    // 1. Create a pre-MST with all the vertices and no edges.
    // 2. Repeat:
    //
    //  a. Remove the shortest edge from the original graph.
    //  b. If the edge connects two disjoint trees in the pre-MST,
    //     add the edge.
    while let Some(MinScored(score, (a, b))) = sort_edges.pop() {
        // check if the edge would connect two disjoint parts
        if subgraphs.union(a.index(), b.index()) {
            mst.add_edge(a, b, score);
        }
    }

    debug_assert!(mst.node_count() == g.node_count());
    debug_assert!(mst.edge_count() < g.node_count());
    mst
}
