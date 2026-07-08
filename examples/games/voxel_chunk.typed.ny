import "stdlib/games/voxel.ny"

fn main() -> void {
    let mut chunk: VoxelChunk_i32 = VoxelChunk_i32_new(8, 0)
    chunk = chunk.set(0, 0, 0, 2)
    chunk = chunk.set(1, 0, 0, 1)
    print(chunk.solid_count(), chunk.visible_face_count())
}
