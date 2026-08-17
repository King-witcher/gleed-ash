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

struct Transforms
{
    float4x4 model;
    float4x4 camera;
}

// [binding, set]
[[vk::binding(0, 0)]]
ConstantBuffer<Transforms> transforms;

[shader("vertex")]
VertexOutput vertMain(VertexInput input)
{
    VertexOutput output;
    output.position = mul(transforms.camera, mul(transforms.model, float4(input.position, 1.0)));
    output.color = input.color;
    return output;
}

[shader("fragment")]
float4 fragMain(VertexOutput vertexOutput) : SV_Target
{
    return float4(vertexOutput.color, 1.0);
}
