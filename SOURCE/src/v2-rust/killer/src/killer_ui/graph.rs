//! **Phase C** — operator graph (DAG): dataflow nodes with topological cook order.

use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

/// High-level operator kinds (expand like TOP/CHOP/DAT *ideas*, not TD-specific).
#[derive(Debug, Clone)]
pub enum OperatorKind {
    ConstantFloat(f64),
    FloatArray(Vec<f64>),
    Add,
    Mul,
    /// Placeholder bridge to procedural texture metadata (e.g. nova_gen size).
    TextureMeta { width: u32, height: u32 },
    DatTablePlaceholder,
}

#[derive(Debug, Clone)]
pub struct OperatorNode {
    pub id: NodeId,
    pub kind: OperatorKind,
    /// Upstream producers this node depends on (they must cook before this node).
    pub inputs: Vec<NodeId>,
}

#[derive(Debug, Clone, Default)]
pub struct OperatorGraph {
    pub nodes: Vec<OperatorNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CookError {
    UnknownNode(String),
    CycleDetected,
}

impl OperatorGraph {
    pub fn demo_c() -> Self {
        Self {
            nodes: vec![
                OperatorNode {
                    id: NodeId("a".into()),
                    kind: OperatorKind::ConstantFloat(1.0),
                    inputs: vec![],
                },
                OperatorNode {
                    id: NodeId("b".into()),
                    kind: OperatorKind::ConstantFloat(2.0),
                    inputs: vec![],
                },
                OperatorNode {
                    id: NodeId("sum".into()),
                    kind: OperatorKind::Add,
                    inputs: vec![NodeId("a".into()), NodeId("b".into())],
                },
            ],
        }
    }

    /// Deterministic cook order: producers before consumers (Kahn topological sort).
    pub fn topo_cook_order(&self) -> Result<Vec<NodeId>, CookError> {
        let n = self.nodes.len();
        let mut id_to_idx: HashMap<String, usize> = HashMap::with_capacity(n);
        for (i, node) in self.nodes.iter().enumerate() {
            id_to_idx.insert(node.id.0.clone(), i);
        }
        let mut indegree: Vec<usize> = vec![0; n];
        // adj[i] = downstream nodes that depend on node i
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for (j, node) in self.nodes.iter().enumerate() {
            for inp in &node.inputs {
                let _ = id_to_idx
                    .get(&inp.0)
                    .ok_or_else(|| CookError::UnknownNode(inp.0.clone()))?;
            }
            indegree[j] = node.inputs.len();
            for inp in &node.inputs {
                let i = id_to_idx[&inp.0];
                adj[i].push(j);
            }
        }

        let mut starters: Vec<usize> = indegree
            .iter()
            .enumerate()
            .filter(|(_, &d)| d == 0)
            .map(|(i, _)| i)
            .collect();
        starters.sort_by(|&i, &k| self.nodes[i].id.0.cmp(&self.nodes[k].id.0));

        let mut q: VecDeque<usize> = starters.into_iter().collect();
        let mut out: Vec<NodeId> = Vec::with_capacity(n);
        while let Some(i) = q.pop_front() {
            out.push(NodeId(self.nodes[i].id.0.clone()));
            let mut next: Vec<usize> = Vec::new();
            for &j in &adj[i] {
                indegree[j] -= 1;
                if indegree[j] == 0 {
                    next.push(j);
                }
            }
            next.sort_by(|&j, &k| self.nodes[j].id.0.cmp(&self.nodes[k].id.0));
            for j in next {
                q.push_back(j);
            }
        }

        if out.len() != n {
            return Err(CookError::CycleDetected);
        }
        Ok(out)
    }

    /// Naive numeric cook: Constants, Add, Mul.
    pub fn cook_floats(&self) -> Result<HashMap<String, f64>, CookError> {
        let order = self.topo_cook_order()?;
        let mut vals: HashMap<String, f64> = HashMap::new();
        for nid in order {
            let node = self
                .nodes
                .iter()
                .find(|n| n.id.0 == nid.0)
                .ok_or_else(|| CookError::UnknownNode(nid.0.clone()))?;
            let v = match &node.kind {
                OperatorKind::ConstantFloat(x) => *x,
                OperatorKind::FloatArray(a) => {
                    if a.is_empty() {
                        0.0
                    } else {
                        a.iter().copied().sum::<f64>() / a.len() as f64
                    }
                }
                OperatorKind::Add => node
                    .inputs
                    .iter()
                    .map(|i| vals.get(&i.0).copied().unwrap_or(0.0))
                    .sum(),
                OperatorKind::Mul => {
                    let ps: Vec<f64> = node
                        .inputs
                        .iter()
                        .map(|i| vals.get(&i.0).copied().unwrap_or(1.0))
                        .collect();
                    if ps.is_empty() {
                        0.0
                    } else {
                        ps.iter().product()
                    }
                }
                OperatorKind::TextureMeta { .. } | OperatorKind::DatTablePlaceholder => 0.0,
            };
            vals.insert(node.id.0.clone(), v);
        }
        Ok(vals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topo_and_cook_demo() {
        let g = OperatorGraph::demo_c();
        let order = g.topo_cook_order().expect("topo");
        assert!(order.iter().any(|n| n.0 == "sum"));
        let vals = g.cook_floats().expect("cook");
        assert!((vals["sum"] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn cycle_fails() {
        let g = OperatorGraph {
            nodes: vec![
                OperatorNode {
                    id: NodeId("x".into()),
                    kind: OperatorKind::Add,
                    inputs: vec![NodeId("y".into())],
                },
                OperatorNode {
                    id: NodeId("y".into()),
                    kind: OperatorKind::Add,
                    inputs: vec![NodeId("x".into())],
                },
            ],
        };
        assert_eq!(g.topo_cook_order(), Err(CookError::CycleDetected));
    }
}
