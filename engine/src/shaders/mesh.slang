struct VertexInput
{
    [[vk::location(0)]]
    float3 position;

    [[vk::location(1)]]
    float3 color;
}

struct VertexOutput
{
    [[vk::location(0)]]
    float3 color;
    float4 position : SV_Position;
}

struct PushConstants
{
    float4x4 model;
}

// [binding, descriptor set]
[[vk::binding(0, 0)]]
ConstantBuffer<float4x4> camera;

[[vk::push_constant]]
ConstantBuffer<PushConstants> pc;

[shader("vertex")]
VertexOutput vertMain(VertexInput input)
{
    VertexOutput output;
    output.position = mul(camera, mul(pc.model, float4(input.position, 1.0)));
    output.color = input.color;
    return output;
}

[shader("fragment")]
float4 fragMain(VertexOutput vertexOutput) : SV_Target
{
    return float4(vertexOutput.color, 1.0);
}
