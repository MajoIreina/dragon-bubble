extends Node2D

## Small, fading accents stay in the pet window and follow the actual head.
var actor: PetAvatar
var clock := 0.0
var sleep_opacity := 0.0
var idle_opacity := 0.0

func advance(delta: float) -> void:
	clock += delta
	sleep_opacity = move_toward(sleep_opacity, 1.0 if actor.state == "sleep" else 0.0, delta * 2.0)
	idle_opacity = move_toward(idle_opacity, 1.0 if actor.state == "idle" else 0.0, delta * 3.0)
	queue_redraw()

func sleep_particles() -> Array[Dictionary]:
	var particles: Array[Dictionary] = []
	for i in 3:
		var phase := fposmod(clock / 3.6 + i / 3.0, 1.0)
		particles.append({"position": actor.effect_anchor + Vector2(18 + phase * 25 + sin(phase * TAU) * 4, -12 - phase * 53),
			"alpha": sin(phase * PI) * sleep_opacity * 0.82, "size": 13 + roundi(phase * 9)})
	return particles

func _draw() -> void:
	if not is_instance_valid(actor):
		return
	var accent := Color("83b8c2")
	if sleep_opacity > 0.001:
		for particle in sleep_particles():
			var at: Vector2 = particle.position.clamp(Vector2(16, 30), Vector2(287, 200))
			draw_string(ThemeDB.fallback_font, at, "z" if particle.size < 18 else "Z", HORIZONTAL_ALIGNMENT_LEFT, -1, particle.size, Color(accent, particle.alpha))
		var breath := 0.5 - cos(clock * TAU / 5.5) * 0.5
		var center := actor.effect_anchor + Vector2(13, 11)
		var radius := 3.0 + breath * 5.0
		draw_circle(center, radius, Color(0.83, 0.94, 0.97, sleep_opacity * 0.15))
		draw_arc(center, radius, 0, TAU, 32, Color(accent, sleep_opacity * 0.5), 1.2, true)
		draw_arc(center + Vector2(-1, -1), radius * 0.52, PI, PI * 1.45, 12, Color(1, 1, 1, sleep_opacity * 0.8), 1.3, true)
	# A brief pair of soft glints accent the idle glance, with long quiet gaps.
	var pulse := maxf(0.0, 1.0 - absf(fposmod(actor.elapsed, 6.0) - 4.6) / 0.6) * idle_opacity
	if pulse > 0.001:
		for i in 2:
			var at := actor.effect_anchor + Vector2(-26 + i * 52, -12 - i * 10)
			var width := (2.0 + i) * pulse
			draw_line(at - Vector2(width, 0), at + Vector2(width, 0), Color(accent, pulse * 0.7), 1.2, true)
			draw_line(at - Vector2(0, width), at + Vector2(0, width), Color(accent, pulse * 0.7), 1.2, true)
