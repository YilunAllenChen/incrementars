use std::collections::HashMap;

type NodeFunc = fn(f32) -> f32;
type NodeId = usize;

pub struct Node {
    pub id: NodeId,
    pub val: f32,
    pub dirty: bool,
    pub children: Vec<NodeId>,
    pub parents: Vec<NodeId>,
    pub func: NodeFunc,
}

impl Node {
    pub fn new(id: NodeId, func: NodeFunc) -> Self {
        Node {
            id,
            val: 0.0,
            dirty: false,
            children: vec![],
            parents: vec![],
            func,
        }
    }
}

pub fn link(parent: &mut Node, child: &mut Node) {
    parent.children.push(child.id);
    child.parents.push(parent.id);
}

pub struct Graph {
    pub nodes: HashMap<NodeId, Node>,
}

impl Graph {
    pub fn set(&mut self, node_id: NodeId, val: f32) {
        let node = self.nodes.get_mut(&node_id).unwrap();
        node.val = val;

        let mut stack = node.children.clone();
        while let Some(node_id) = stack.pop() {
            let dnode = self.nodes.get_mut(&node_id).unwrap();
            dnode.dirty = true;
            stack.extend(dnode.children.clone())
        }
    }

    pub fn stabilize(&mut self) {
        let nodes = &mut self.nodes;
        let mut stack: Vec<NodeId> = nodes
            .iter()
            .filter_map(|(id, node)| if node.dirty { Some(id.clone()) } else { None })
            .collect();

        let mut new_vals: HashMap<NodeId, f32> = HashMap::new();
        while let Some(node_id) = stack.pop() {
            let num_dirty_parents =
                nodes
                    .get(&node_id)
                    .unwrap()
                    .parents
                    .iter()
                    .fold(0, |acc, pa_id| match nodes.get(pa_id).unwrap().dirty {
                        true => acc + 1,
                        false => acc,
                    });

            if num_dirty_parents > 0 {
                stack.push(node_id);
            } else {
                let curr = nodes.get(&node_id).unwrap();
                let pa_id = curr.parents[0];
                let pa_val = nodes.get(&pa_id).unwrap().val;
                new_vals.insert(node_id, (curr.func)(pa_val));
            }
        }

        new_vals.into_iter().for_each(|(node_id, new_val)| {
            let curr = nodes.get_mut(&node_id).unwrap();
            println!("setting {}: {} -> {}", node_id, curr.val, new_val);
            curr.val = new_val;
        })
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_add() {
        let mut node0 = Node::new(0, |x| x * 2.0);
        let mut node1 = Node::new(1, |x| x * 4.0);

        link(&mut node0, &mut node1);

        let mut g = Graph {
            nodes: HashMap::from_iter(vec![(0, node0), (1, node1)]),
        };

        g.set(0, 2.0);

        assert_eq!(g.nodes.get(&1).unwrap().dirty, true)
    }

    #[test]
    fn test_propagate() {
        let mut node0 = Node::new(0, |x| x * 2.0);
        let mut node1 = Node::new(1, |x| x * 4.0);

        link(&mut node0, &mut node1);
        let mut g = Graph {
            nodes: HashMap::from_iter(vec![(0, node0), (1, node1)]),
        };

        g.set(0, 2.0);
        g.stabilize();
        assert_eq!(g.nodes.get(&1).unwrap().val, 8.0)
    }
}
