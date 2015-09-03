
use fb::FixedBitSet;
use std::default::Default;
use {Graph, Directed, Undirected, EdgeType};
use graph::NodeIndex;

/*
pub struct DAG {
    size: usize,
    bits: FixedBitSet,
}

// A DAG has the property that the adjacency matrix is lower triangular,
// diagonal zero.
//
// This means we only allow edges i → j where i < j.
//
// The set of all DAG of a particular size is simply the power set of all
// possible edges.
//
// For a graph of n=3 nodes we have (n - 1) * n / 2 = 3 possible edges.
//
// Gray code
//
// gray(x) { x ^ (x >> 1) }
//
// See fxtbook on gray codes
//
// Use a gray code sequence to efficiently step through the whole set of edges

impl DAG {
    pub fn new(size: usize) -> Self {
        DAG {
            size: size,
            bits: FixedBitSet::with_capacity(size),
        }
    }

    fn state_to_graph(&self) -> Graph<(), (), Directed> {
        let popcount = self.bits.as_slice().iter()
                                .fold(0, |acc, x| acc + x.count_ones() as usize);
        Graph::with_capacity(self.size, popcount)
    }
}
*/

pub struct Generator<Ty> {
    acyclic: bool,
    selfloops: bool,
    nodes: usize,
    /// number of possible edges
    nedges: usize,
    /// current edge bitmap
    bits: u64,
    g: Graph<(), (), Ty>,
}

impl Generator<Directed> {
    /// Generate all possible Directed acyclic graphs (DAGs) of a particular number of vertices.
    ///
    /// For a graph of *k* vertices there are *e = (k - 1) k / 2* possible edges and
    /// *2<sup>e</sup>* DAGs.
    pub fn directed_acyclic(nodes: usize) -> Self {
        assert!(nodes != 0);
        let nedges = (nodes - 1) * nodes / 2;
        assert!(nedges < 64);
        Generator {
            acyclic: true,
            selfloops: false,
            nodes: nodes,
            nedges:nedges,
            bits: !0,
            g: Graph::with_capacity(nodes, nedges),
        }
    }
}

impl<Ty: EdgeType> Generator<Ty> {
    /// Generate all possible graphs of a particular number of vertices.
    ///
    /// For a graph of *k* vertices there are *e = k²* possible edges and
    /// *2<sup>k<sup>2</sup></sup>* graphs.
    pub fn all(nodes: usize, allow_selfloops: bool) -> Self {
        let nedges = if allow_selfloops {
            nodes * nodes
        } else {
            (nodes * nodes) - nodes
        };
        assert!(nedges < 64);
        Generator {
            acyclic: false,
            selfloops: allow_selfloops,
            nodes: nodes,
            nedges: nedges,
            bits: !0,
            g: Graph::with_capacity(nodes, nedges),
        }
    }

    fn state_to_graph(&mut self) -> &Graph<(), (), Ty> {
        let popcount = self.bits.count_ones() as usize;
        self.g.clear();
        for _ in 0..self.nodes {
            self.g.add_node(());
        }
        // For a DAG:
        // interpret the bits in order, it's a lower triangular matrix:
        //   a b c d
        // a x x x x
        // b 0 x x x
        // c 1 2 x x
        // d 3 4 5 x
        let mut bit = 0;
        for i in 0..self.nodes {
            let start = if self.acyclic || !self.g.is_directed() { i } else { 0 };
            for j in start..self.nodes {
                if i == j && !self.selfloops {
                    continue;
                }
                if self.bits & (1u64 << bit) != 0 {
                    self.g.add_edge(NodeIndex::new(i), NodeIndex::new(j), ());
                }

                bit += 1;
            }
        }
        &self.g
    }

    pub fn next_ref(&mut self) -> Option<&Graph<(), (), Ty>> {
        if self.bits == !0 {
            self.bits = 0;
        } else {
            self.bits += 1;
            if self.bits >= 1u64 << self.nedges {
                return None;
            }
        }
        Some(self.state_to_graph())
    }
}

impl<Ty: EdgeType> Iterator for Generator<Ty> {
    type Item = Graph<(), (), Ty>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_ref().cloned()
    }
}
