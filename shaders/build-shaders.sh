SHADER_DIR=$(dirname $0)

dxc -spirv -E main -T vs_6_1 -Fo $SHADER_DIR/vertex.spv $SHADER_DIR/vertex.vert
dxc -spirv -E main -T ps_6_1 -Fo $SHADER_DIR/fragment.spv $SHADER_DIR/fragment.frag
