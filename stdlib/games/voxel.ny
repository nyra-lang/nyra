import "../vec.ny"

struct VoxelChunk_i32 {
    size: i32
    blocks: ptr
}

fn VoxelChunk_i32_index(size, x, y, z) {
    return x + z * size + y * size * size
}

fn VoxelChunk_i32_in_bounds(size, x, y, z) {
    if x < 0 || y < 0 || z < 0 {
        return 0
    }
    if x >= size || y >= size || z >= size {
        return 0
    }
    return 1
}

fn VoxelChunk_i32_new(size, fill) {
    let n = size * size * size
    let blocks = Vec_i32_new()
    let mut i = 0
    while i < n {
        Vec_i32_push(blocks, fill)
        i = i + 1
    }
    return VoxelChunk_i32 { size: size, blocks: blocks }
}

fn VoxelChunk_i32_neighbor_air(chunk: VoxelChunk_i32, _x: i32, _y: i32, _z: i32, nx: i32, ny: i32, nz: i32) -> i32 {
    if VoxelChunk_i32_in_bounds(chunk.size, nx, ny, nz) == 0 {
        return 1
    }
    if Vec_i32_get(chunk.blocks, VoxelChunk_i32_index(chunk.size, nx, ny, nz)) == 0 {
        return 1
    }
    return 0
}

impl VoxelChunk_i32 {
    fn get(self, x: i32, y: i32, z: i32) -> i32 {
        if VoxelChunk_i32_in_bounds(self.size, x, y, z) == 0 {
            return 0
        }
        let idx = VoxelChunk_i32_index(self.size, x, y, z)
        return Vec_i32_get(self.blocks, idx)
    }

    fn set(self, x: i32, y: i32, z: i32, value: i32) -> VoxelChunk_i32 {
        if VoxelChunk_i32_in_bounds(self.size, x, y, z) == 0 {
            return self
        }
        let idx = VoxelChunk_i32_index(self.size, x, y, z)
        Vec_i32_set(self.blocks, idx, value)
        return self
    }

    fn solid_count(self) -> i32 {
        let n = self.size * self.size * self.size
        let mut count = 0
        let mut i = 0
        while i < n {
            if Vec_i32_get(self.blocks, i) != 0 {
                count = count + 1
            }
            i = i + 1
        }
        return count
    }

    fn visible_face_count(self) -> i32 {
        let mut faces = 0
        let mut x = 0
        while x < self.size {
            let mut y = 0
            while y < self.size {
                let mut z = 0
                while z < self.size {
                    if self.get(x, y, z) != 0 {
                        if VoxelChunk_i32_neighbor_air(self, x, y, z, x + 1, y, z) == 1 {
                            faces = faces + 1
                        }
                        if VoxelChunk_i32_neighbor_air(self, x, y, z, x - 1, y, z) == 1 {
                            faces = faces + 1
                        }
                        if VoxelChunk_i32_neighbor_air(self, x, y, z, x, y + 1, z) == 1 {
                            faces = faces + 1
                        }
                        if VoxelChunk_i32_neighbor_air(self, x, y, z, x, y - 1, z) == 1 {
                            faces = faces + 1
                        }
                        if VoxelChunk_i32_neighbor_air(self, x, y, z, x, y, z + 1) == 1 {
                            faces = faces + 1
                        }
                        if VoxelChunk_i32_neighbor_air(self, x, y, z, x, y, z - 1) == 1 {
                            faces = faces + 1
                        }
                    }
                    z = z + 1
                }
                y = y + 1
            }
            x = x + 1
        }
        return faces
    }
}

// Aliases for tests/examples that prefer free functions.
fn VoxelChunk_i32_get(chunk: VoxelChunk_i32, x: i32, y: i32, z: i32) -> i32 {
    return chunk.get(x, y, z)
}

fn VoxelChunk_i32_set(chunk: VoxelChunk_i32, x: i32, y: i32, z: i32, value: i32) -> VoxelChunk_i32 {
    return chunk.set(x, y, z, value)
}

fn VoxelChunk_i32_solid_count(chunk: VoxelChunk_i32) -> i32 {
    return chunk.solid_count()
}

fn VoxelChunk_i32_visible_face_count(chunk: VoxelChunk_i32) -> i32 {
    return chunk.visible_face_count()
}

impl Drop for VoxelChunk_i32 {
    fn drop(self) -> void {
        Vec_i32_free(self.blocks)
    }
}
