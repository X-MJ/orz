use super::auxility::UncheckedSliceExt;
use super::bits::Bits;

pub struct HuffmanEncoder {
    canonical_lens: Vec<u8>,
    encodings: Vec<u16>,
}

pub struct HuffmanDecoder {
    canonical_lens: Vec<u8>,
    canonical_lens_max: u8,
    decodings: Vec<u16>,
}

impl HuffmanEncoder {
    pub fn from_symbol_weights(symbol_weights: &[u32], canonical_lens_max: u8) -> HuffmanEncoder {
        let canonical_lens = compute_canonical_lens(symbol_weights, canonical_lens_max);
        let encodings = compute_encodings(&canonical_lens);
        return HuffmanEncoder {
            canonical_lens,
            encodings,
        };
    }

    pub fn get_canonical_lens(&self) -> &[u8] {
        return &self.canonical_lens;
    }

    pub unsafe fn encode_to_bits(&self, symbol: u16, bits: &mut Bits) {
        let bits_len = self.canonical_lens.nocheck()[symbol as usize];
        let bs = self.encodings.nocheck()[symbol as usize];
        bits.put(bits_len, bs as u64);
    }
}

impl HuffmanDecoder {
    pub fn from_canonical_lens(canonical_lens: &[u8]) -> HuffmanDecoder {
        let canonical_lens_max = *canonical_lens.iter().max().unwrap();
        let encodings = compute_encodings(canonical_lens);
        let decodings = compute_decodings(canonical_lens, &encodings, canonical_lens_max);
        return HuffmanDecoder {
            canonical_lens: Vec::from(canonical_lens),
            canonical_lens_max,
            decodings,
        };
    }

    pub unsafe fn decode_from_bits(&self, bits: &mut Bits) -> u16 {
        let symbol = self.decodings.nocheck()[bits.peek(self.canonical_lens_max) as usize];
        bits.skip(self.canonical_lens.nocheck()[symbol as usize]);
        return symbol;
    }
}

fn compute_canonical_lens(symbol_weights: &[u32], canonical_lens_max: u8) -> Vec<u8> {
    #[derive(Ord, Eq, PartialOrd, PartialEq)]
    struct Node {
        weight: i64,
        symbol: u16,
        child1: Option<Box<Node>>,
        child2: Option<Box<Node>>,
    };

    'shrink: for shrink_factor in 0 .. {
        let mut canonical_lens = vec![0u8; match symbol_weights.len() % 2 {
            0 => symbol_weights.len(),
            _ => symbol_weights.len() + 1,
        }];

        let mut node_heap = symbol_weights.iter().enumerate().filter_map(|(symbol, &weight)| {
            match weight {
                0 => None,
                _ => Some(Box::new(Node {
                    weight: -std::cmp::max(weight as i64 / (1 << shrink_factor), 1),
                    symbol: symbol as u16,
                    child1: None,
                    child2: None,
                })),
            }
        }).collect::<std::collections::BinaryHeap<_>>();

        if node_heap.len() < 2 {
            if node_heap.len() == 1 {
                canonical_lens[node_heap.pop().unwrap().symbol as usize] = 1;
            }
            return canonical_lens;
        }

        // construct huffman tree
        while node_heap.len() > 1 {
            let min_node1 = node_heap.pop().unwrap();
            let min_node2 = node_heap.pop().unwrap();
            node_heap.push(Box::new(Node {
                weight: min_node1.weight + min_node2.weight,
                symbol: u16::max_value(),
                child1: Some(min_node1),
                child2: Some(min_node2),
            }));
        }

        // iterate huffman tree and extract symbol bits length
        let root_node = node_heap.pop().unwrap();
        let mut nodes_iterator_queue = vec![(0, &root_node)];
        while !nodes_iterator_queue.is_empty() {
            let (depth, node) = nodes_iterator_queue.pop().unwrap();
            if node.symbol == u16::max_value() {
                if depth == canonical_lens_max {
                    continue 'shrink;
                }
                nodes_iterator_queue.push((depth + 1, &node.child1.as_ref().unwrap()));
                nodes_iterator_queue.push((depth + 1, &node.child2.as_ref().unwrap()));
            } else {
                canonical_lens[node.symbol as usize] = depth;
            }
        }
        return canonical_lens;
    }
    unreachable!()
}

fn compute_encodings(canonical_lens: &[u8]) -> Vec<u16> {
    let mut encodings = vec![0u16; canonical_lens.len()];
    let mut bits: u16 = 0;
    let mut current_bits_len: u8 = 1;

    let ordered_symbol_with_bits_lens = canonical_lens.iter().enumerate().filter_map(|(symbol, &bits_len)| {
        match bits_len {
            0 => None,
            _ => Some((bits_len, symbol as u16)),
        }
    }).collect::<std::collections::BTreeSet<_>>();

    ordered_symbol_with_bits_lens.iter().for_each(|symbol_with_bits_len| {
        while current_bits_len < symbol_with_bits_len.0 {
            bits <<= 1;
            current_bits_len += 1;
        }
        encodings[symbol_with_bits_len.1 as usize] = bits;
        bits += 1;
    });
    return encodings;
}

fn compute_decodings(canonical_lens: &[u8], encodings: &[u16], canonical_lens_max: u8) -> Vec<u16> {
    let mut decodings = vec![0u16; 1 << canonical_lens_max];
    for symbol in 0..canonical_lens.len() {
        unsafe {
            if canonical_lens.nocheck()[symbol as usize] > 0 {
                let rest_bits_len = canonical_lens_max - canonical_lens.nocheck()[symbol as usize];
                let blo = (encodings.nocheck()[symbol as usize] + 0) << rest_bits_len;
                let bhi = (encodings.nocheck()[symbol as usize] + 1) << rest_bits_len;
                for b in blo..bhi {
                    decodings.nocheck_mut()[b as usize] = symbol as u16;
                }
            }
        }
    }
    return decodings;
}
