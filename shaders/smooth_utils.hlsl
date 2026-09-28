// -*- mode: hlsl -*-

float smoothed_pulse(float value, float center, float half_thickness) {
    float distance_from_center = abs(value - center);

    float change_per_pixel = fwidth(distance_from_center);
    float change_per_half_pixel = change_per_pixel * 0.5;

    float lower_pixel_boundary = half_thickness - change_per_half_pixel;
    float upper_pixel_boundary = half_thickness + change_per_half_pixel;

    return 1.0 - smoothstep(
        lower_pixel_boundary,
        upper_pixel_boundary,
        distance_from_center
    );
}

float smoothed_triangle_wave(float value) {
    float repeating_sawtooth = frac(value) * 2.0;
    float continuous_triangle = 1.0 - abs(repeating_sawtooth - 1.0);

    return continuous_triangle;
}

float smooth_modulo(float value, float period) {
    float progress = value / period;

    return smoothed_triangle_wave(progress);
}

float smoothed_condition(float value, float flip_at) {
    float change_per_pixel = fwidth(value);
    float change_per_half_pixel = change_per_pixel * 0.5;

    float transition_from = flip_at - change_per_half_pixel;
    float transition_to = flip_at + change_per_half_pixel;

    return smoothstep(
        transition_from,
        transition_to,
        value
    );
}

float smoothed_circle(float2 uv, float2 center, float radius) {
    float distance_from_center = distance(uv, center);

    return 1.0 - smoothed_condition(distance_from_center, radius);
}
