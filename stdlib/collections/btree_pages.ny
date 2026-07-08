// Page-based B-tree — node pool in HashMap (survives struct move).
// Internal descent, leaf + internal splits, production fanout (BTREE_PAGE_MAX keys per node).

const BTREE_PAGE_MAX = 8

struct BTreePaged_str_str {
    root: i32
    nodes: HashMap_str_str
    next_id: i32
}

struct BTreeSplitResult {
    tree: BTreePaged_str_str
    promote: string
    right_id: i32
    did_split: i32
}

fn BTreePaged_new() -> BTreePaged_str_str {
    return BTreePaged_str_str { root: 0, nodes: HashMap_str_str_new(), next_id: 0 }
}

fn BTreePaged_node_key(id: i32) -> string {
    return strcat("n:", i32_to_string(id))
}

fn BTreePaged_join(vec: StrVec) -> string {
    let n = vec.len()
    if n == 0 {
        return ""
    }
    let mut out = vec.get(0)
    let mut i = 1
    while i < n {
        out = strcat(out, "\x1f")
        out = strcat(out, vec.get(i))
        i = i + 1
    }
    return out
}

fn BTreePaged_split_field(text: string) -> StrVec {
    if strlen(text) == 0 {
        return StrVec_new()
    }
    return StrVec { handle: String_split(text, "\x1f") }
}

fn BTreePaged_node_line(tree: BTreePaged_str_str, id: i32) -> string {
    return tree.nodes.get(BTreePaged_node_key(id))
}

fn BTreePaged_put_line(tree: BTreePaged_str_str, id: i32, line: string) -> BTreePaged_str_str {
    return BTreePaged_str_str { root: tree.root, nodes: tree.nodes.insert(BTreePaged_node_key(id), line), next_id: tree.next_id }
}

fn BTreePaged_add_line(tree: BTreePaged_str_str, line: string) -> BTreePaged_str_str {
    let id = tree.next_id
    let next = BTreePaged_put_line(tree, id, line)
    return BTreePaged_str_str { root: next.root, nodes: next.nodes, next_id: id + 1 }
}

fn BTreePaged_pack_leaf(keys: StrVec, values: StrVec) -> string {
    return strcat("L|", strcat(BTreePaged_join(keys), strcat("|", strcat(BTreePaged_join(values), "|"))))
}

fn BTreePaged_pack_internal(keys: StrVec, children: StrVec) -> string {
    return strcat("I|", strcat(BTreePaged_join(keys), strcat("|", strcat(BTreePaged_join(children), "|"))))
}

fn BTreePaged_leaf_fields(line: string) -> StrVec {
    let p1 = strstr_pos(line, "|")
    let rest = substring(line, p1 + 1, strlen(line) - p1 - 1)
    let p2 = strstr_pos(rest, "|")
    let keys_csv = substring(rest, 0, p2)
    let rest2 = substring(rest, p2 + 1, strlen(rest) - p2 - 1)
    let p3 = strstr_pos(rest2, "|")
    let vals_csv = substring(rest2, 0, p3)
    let mut out = StrVec_new()
    out = out.push(keys_csv)
    out = out.push(vals_csv)
    return out
}

fn BTreePaged_internal_fields(line: string) -> StrVec {
    let p1 = strstr_pos(line, "|")
    let rest = substring(line, p1 + 1, strlen(line) - p1 - 1)
    let p2 = strstr_pos(rest, "|")
    let keys_csv = substring(rest, 0, p2)
    let rest2 = substring(rest, p2 + 1, strlen(rest) - p2 - 1)
    let p3 = strstr_pos(rest2, "|")
    let children_csv = substring(rest2, 0, p3)
    let mut out = StrVec_new()
    out = out.push(keys_csv)
    out = out.push(children_csv)
    return out
}

fn BTreePaged_find_in_keys(keys: StrVec, key: string) -> i32 {
    let n = keys.len()
    let mut lo = 0
    let mut hi = n
    while lo < hi {
        let mid = (lo + hi) / 2
        let cmp = strcmp(keys.get(mid), key)
        if cmp == 0 {
            return mid
        }
        if cmp < 0 {
            lo = mid + 1
        } else {
            hi = mid
        }
    }
    return -lo - 1
}

fn BTreePaged_child_index(keys: StrVec, key: string) -> i32 {
    let n = keys.len()
    let mut i = 0
    while i < n {
        if strcmp(key, keys.get(i)) < 0 {
            return i
        }
        i = i + 1
    }
    return n
}

fn StrVec_insert_at(vec: StrVec, at: i32, value: string) -> StrVec {
    let mut out = StrVec_new()
    let n = vec.len()
    let mut i = 0
    while i < n {
        if i == at {
            out = out.push(value)
        }
        out = out.push(vec.get(i))
        i = i + 1
    }
    if at == n {
        out = out.push(value)
    }
    return out
}

fn BTreePaged_no_split(tree: BTreePaged_str_str) -> BTreeSplitResult {
    return BTreeSplitResult { tree: tree, promote: "", right_id: 0, did_split: 0 }
}

fn BTreePaged_leaf_insert(tree: BTreePaged_str_str, node_id: i32, key: string, value: string) -> BTreeSplitResult {
    let line = BTreePaged_node_line(tree, node_id)
    let fields = BTreePaged_leaf_fields(line)
    let keys = BTreePaged_split_field(fields.get(0))
    let vals = BTreePaged_split_field(fields.get(1))
    let idx = BTreePaged_find_in_keys(keys, key)
    if idx >= 0 {
        let mut new_vals = StrVec_new()
        let n = vals.len()
        let mut i = 0
        while i < n {
            if i == idx {
                new_vals = new_vals.push(value)
            } else {
                new_vals = new_vals.push(vals.get(i))
            }
            i = i + 1
        }
        let next = BTreePaged_put_line(tree, node_id, BTreePaged_pack_leaf(keys, new_vals))
        return BTreePaged_no_split(next)
    }
    let at = -idx - 1
    let new_keys = StrVec_insert_at(keys, at, key)
    let new_vals = StrVec_insert_at(vals, at, value)
    if new_keys.len() <= BTREE_PAGE_MAX {
        let next = BTreePaged_put_line(tree, node_id, BTreePaged_pack_leaf(new_keys, new_vals))
        return BTreePaged_no_split(next)
    }
    let mid = BTREE_PAGE_MAX / 2
    let mut left_keys = StrVec_new()
    let mut left_vals = StrVec_new()
    let mut right_keys = StrVec_new()
    let mut right_vals = StrVec_new()
    let mut i = 0
    while i < new_keys.len() {
        if i <= mid {
            left_keys = left_keys.push(new_keys.get(i))
            left_vals = left_vals.push(new_vals.get(i))
        } else {
            right_keys = right_keys.push(new_keys.get(i))
            right_vals = right_vals.push(new_vals.get(i))
        }
        i = i + 1
    }
    let promote = right_keys.get(0)
    let mut right_keys2 = StrVec_new()
    let mut right_vals2 = StrVec_new()
    let mut j = 1
    while j < right_keys.len() {
        right_keys2 = right_keys2.push(right_keys.get(j))
        right_vals2 = right_vals2.push(right_vals.get(j))
        j = j + 1
    }
    let mut next = BTreePaged_put_line(tree, node_id, BTreePaged_pack_leaf(left_keys, left_vals))
    next = BTreePaged_add_line(next, BTreePaged_pack_leaf(right_keys2, right_vals2))
    let right_id = next.next_id - 1
    return BTreeSplitResult { tree: next, promote: promote, right_id: right_id, did_split: 1 }
}

fn BTreePaged_internal_insert_key(tree: BTreePaged_str_str, node_id: i32, at: i32, promote: string, right_id: i32) -> BTreeSplitResult {
    let line = BTreePaged_node_line(tree, node_id)
    let fields = BTreePaged_internal_fields(line)
    let keys = BTreePaged_split_field(fields.get(0))
    let children = BTreePaged_split_field(fields.get(1))
    let new_keys = StrVec_insert_at(keys, at, promote)
    let new_children = StrVec_insert_at(children, at + 1, i32_to_string(right_id))
    if new_keys.len() <= BTREE_PAGE_MAX {
        let next = BTreePaged_put_line(tree, node_id, BTreePaged_pack_internal(new_keys, new_children))
        return BTreePaged_no_split(next)
    }
    let mid = BTREE_PAGE_MAX / 2
    let promote2 = new_keys.get(mid)
    let mut left_keys = StrVec_new()
    let mut left_children = StrVec_new()
    let mut right_keys = StrVec_new()
    let mut right_children = StrVec_new()
    let mut k = 0
    while k < new_keys.len() {
        if k < mid {
            left_keys = left_keys.push(new_keys.get(k))
        }
        if k > mid {
            right_keys = right_keys.push(new_keys.get(k))
        }
        k = k + 1
    }
    let mut c = 0
    while c < new_children.len() {
        if c <= mid {
            left_children = left_children.push(new_children.get(c))
        } else {
            right_children = right_children.push(new_children.get(c))
        }
        c = c + 1
    }
    let mut next = BTreePaged_put_line(tree, node_id, BTreePaged_pack_internal(left_keys, left_children))
    next = BTreePaged_add_line(next, BTreePaged_pack_internal(right_keys, right_children))
    let right_id2 = next.next_id - 1
    return BTreeSplitResult { tree: next, promote: promote2, right_id: right_id2, did_split: 1 }
}

fn BTreePaged_insert_node(tree: BTreePaged_str_str, node_id: i32, key: string, value: string) -> BTreeSplitResult {
    let line = BTreePaged_node_line(tree, node_id)
    if char_at(line, 0) == 76 {
        return BTreePaged_leaf_insert(tree, node_id, key, value)
    }
    let fields = BTreePaged_internal_fields(line)
    let keys = BTreePaged_split_field(fields.get(0))
    let children = BTreePaged_split_field(fields.get(1))
    let child_at = BTreePaged_child_index(keys, key)
    let child_id = atoi(children.get(child_at))
    let child_result = BTreePaged_insert_node(tree, child_id, key, value)
    if child_result.did_split == 0 {
        return BTreePaged_no_split(child_result.tree)
    }
    return BTreePaged_internal_insert_key(child_result.tree, node_id, child_at, child_result.promote, child_result.right_id)
}

fn BTreePaged_insert(tree: BTreePaged_str_str, key: string, value: string) -> BTreePaged_str_str {
    if tree.next_id == 0 {
        let leaf = BTreePaged_pack_leaf(StrVec_new().push(key), StrVec_new().push(value))
        let added = BTreePaged_add_line(tree, leaf)
        return BTreePaged_str_str { root: 0, nodes: added.nodes, next_id: added.next_id }
    }
    let result = BTreePaged_insert_node(tree, tree.root, key, value)
    if result.did_split == 0 {
        return result.tree
    }
    let mut children = StrVec_new()
    children = children.push(i32_to_string(result.tree.root))
    children = children.push(i32_to_string(result.right_id))
    let internal = BTreePaged_pack_internal(StrVec_new().push(result.promote), children)
    let next = BTreePaged_add_line(result.tree, internal)
    return BTreePaged_str_str { root: next.next_id - 1, nodes: next.nodes, next_id: next.next_id }
}

fn BTreePaged_leaf_get(line: string, key: string) -> string {
    let fields = BTreePaged_leaf_fields(line)
    let keys = BTreePaged_split_field(fields.get(0))
    let vals = BTreePaged_split_field(fields.get(1))
    let idx = BTreePaged_find_in_keys(keys, key)
    if idx < 0 {
        return ""
    }
    return vals.get(idx)
}

fn BTreePaged_get_node(tree: BTreePaged_str_str, node_id: i32, key: string) -> string {
    let line = BTreePaged_node_line(tree, node_id)
    if strlen(line) == 0 {
        return ""
    }
    if char_at(line, 0) == 76 {
        return BTreePaged_leaf_get(line, key)
    }
    let fields = BTreePaged_internal_fields(line)
    let keys = BTreePaged_split_field(fields.get(0))
    let children = BTreePaged_split_field(fields.get(1))
    let child_at = BTreePaged_child_index(keys, key)
    let child_id = atoi(children.get(child_at))
    return BTreePaged_get_node(tree, child_id, key)
}

fn BTreePaged_get(tree: BTreePaged_str_str, key: string) -> string {
    if tree.next_id == 0 {
        return ""
    }
    return BTreePaged_get_node(tree, tree.root, key)
}

fn BTreePaged_node_count(tree: BTreePaged_str_str) -> i32 {
    return tree.next_id
}

struct BTreePagedRange {
    keys: StrVec
    values: StrVec
}

fn BTreePaged_key_in_range(key: string, lo: string, hi: string) -> i32 {
    if strlen(lo) > 0 {
        if strcmp(key, lo) < 0 {
            return 0
        }
    }
    if strlen(hi) > 0 {
        if strcmp(key, hi) > 0 {
            return 0
        }
    }
    return 1
}

fn BTreePaged_leaf_collect(line: string, lo: string, hi: string, keys: StrVec, vals: StrVec) -> BTreePagedRange {
    let fields = BTreePaged_leaf_fields(line)
    let leaf_keys = BTreePaged_split_field(fields.get(0))
    let leaf_vals = BTreePaged_split_field(fields.get(1))
    let n = leaf_keys.len()
    let mut out_keys = keys
    let mut out_vals = vals
    let mut i = 0
    while i < n {
        let k = leaf_keys.get(i)
        if BTreePaged_key_in_range(k, lo, hi) == 1 {
            out_keys = out_keys.push(k)
            out_vals = out_vals.push(leaf_vals.get(i))
        }
        i = i + 1
    }
    return BTreePagedRange { keys: out_keys, values: out_vals }
}

fn BTreePaged_collect_node(tree: BTreePaged_str_str, node_id: i32, lo: string, hi: string, keys: StrVec, vals: StrVec) -> BTreePagedRange {
    let line = BTreePaged_node_line(tree, node_id)
    if strlen(line) == 0 {
        return BTreePagedRange { keys: keys, values: vals }
    }
    if char_at(line, 0) == 76 {
        return BTreePaged_leaf_collect(line, lo, hi, keys, vals)
    }
    let fields = BTreePaged_internal_fields(line)
    let children = BTreePaged_split_field(fields.get(1))
    let mut out_keys = keys
    let mut out_vals = vals
    let n = children.len()
    let mut i = 0
    while i < n {
        let child_id = atoi(children.get(i))
        let part = BTreePaged_collect_node(tree, child_id, lo, hi, out_keys, out_vals)
        out_keys = part.keys
        out_vals = part.values
        i = i + 1
    }
    return BTreePagedRange { keys: out_keys, values: out_vals }
}

fn BTreePaged_range(tree: BTreePaged_str_str, lo: string, hi: string) -> BTreePagedRange {
    if tree.next_id == 0 {
        return BTreePagedRange { keys: StrVec_new(), values: StrVec_new() }
    }
    return BTreePaged_collect_node(tree, tree.root, lo, hi, StrVec_new(), StrVec_new())
}

fn BTreePaged_keys(tree: BTreePaged_str_str) -> StrVec {
    let all = BTreePaged_range(tree, "", "")
    return all.keys
}
