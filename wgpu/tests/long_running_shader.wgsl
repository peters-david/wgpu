@group(0)
@binding(0)
var<storage, read_write> a: array<u32, 256>;

@stage(compute)
@workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) index: u32) {
    var i: u32 = 256u*id.x+index;
    for (var j: u32 = 0u; j < 100000u; j += 1u){
        a[i] += 1u;
    }
}