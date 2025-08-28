use std::f32::consts::PI;

// Add the path to access openmodel
use wink::openmodel::primitives::{Quaternion, Vector};

fn main() {
    println!("Testing Quaternion implementation...");

    // Test 1: Identity quaternion
    let q_identity = Quaternion::identity();
    println!("✓ Identity: {}", q_identity);
    assert_eq!(q_identity.s, 1.0);
    assert_eq!(q_identity.v.x, 0.0);

    // Test 2: From axis-angle
    let axis = Vector::new(0.0, 0.0, 1.0);
    let q_90 = Quaternion::from_axis_angle(axis, PI / 2.0);
    println!("✓ 90° rotation around Z: {}", q_90);

    // Test 3: Vector rotation
    let v = Vector::new(1.0, 0.0, 0.0);
    let rotated = q_90.rotate_vector(&v);
    println!("✓ Rotated (1,0,0) to: ({:.6}, {:.6}, {:.6})", rotated.x, rotated.y, rotated.z);
    
    // Should be approximately (0, 1, 0)
    assert!((rotated.x - 0.0).abs() < 1e-6);
    assert!((rotated.y - 1.0).abs() < 1e-6);
    assert!((rotated.z - 0.0).abs() < 1e-6);

    // Test 4: Quaternion multiplication
    let q1 = Quaternion::identity();
    let q2 = Quaternion::new(0.0, 1.0, 0.0, 0.0);
    let result = q1 * q2;
    println!("✓ Identity * q2 = {}", result);
    assert_eq!(result, q2);

    // Test 5: Normalization
    let q_unnorm = Quaternion::new(2.0, 0.0, 0.0, 0.0);
    let q_norm = q_unnorm.normalize();
    println!("✓ Normalized magnitude: {:.6}", q_norm.magnitude());
    assert!((q_norm.magnitude() - 1.0).abs() < 1e-6);

    // Test 6: SLERP
    let q_start = Quaternion::identity();
    let q_end = Quaternion::from_axis_angle(Vector::new(0.0, 0.0, 1.0), PI / 2.0);
    let q_mid = q_start.slerp(q_end, 0.5);
    println!("✓ SLERP halfway: {}", q_mid);

    // Test 7: Conversions
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let arr: [f32; 4] = q.into();
    let q2 = Quaternion::from(arr);
    println!("✓ Array conversion: {} -> {:?} -> {}", q, arr, q2);
    assert_eq!(q, q2);

    println!("\n🎉 All quaternion tests passed!");
}
