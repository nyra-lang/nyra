// Raylib 3D helpers — requires `link raylib`. Use with vendor `raylib.ny` drawing symbols.
import "gfx3d.ny"

struct Vector3 repr(C) {
    x: f64
    y: f64
    z: f64
}

struct Camera3D repr(C) {
    position: Vector3
    target: Vector3
    up: Vector3
    fovy: f64
    projection: i32
}

struct Color repr(C) {
    r: u8
    g: u8
    b: u8
    a: u8
}

extern fn BeginMode3D(camera: Camera3D) -> void
extern fn EndMode3D() -> void
extern fn DrawCube(position: Vector3, width: f64, height: f64, length: f64, color: Color) -> void
extern fn DrawGrid(slices: i32, spacing: f64) -> void

fn Raylib_vec3(v: Gfx3D_Vec3) {
    return Vector3 { x: v.x, y: v.y, z: v.z }
}

fn Raylib_camera_orbit(target_x, target_y, target_z, distance, yaw_deg, pitch_deg, fovy) {
    let pos = Gfx3D_orbit_position(target_x, target_y, target_z, distance, yaw_deg, pitch_deg)
    return Camera3D {
        position: Raylib_vec3(pos),
        target: Vector3 { x: target_x, y: target_y, z: target_z },
        up: Vector3 { x: 0.0, y: 1.0, z: 0.0 },
        fovy: fovy,
        projection: 0
    }
}

fn Raylib_mode3d_begin(camera: Camera3D) {
    BeginMode3D(camera)
}

fn Raylib_mode3d_end() {
    EndMode3D()
}
