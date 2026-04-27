//! Integration tests for reference frames with character controller.

#[cfg(test)]
mod tests {
    use glam::{Quat, Vec3};
    use std::f32::consts::PI;

    use crate::character::{CharacterController, CharacterInput, ContactState, FrameMotion};
    use crate::reference_frame::{FrameAttachment, FrameResolver, ReferenceFrame};

    const DT: f32 = 0.016;
    const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
    const EPSILON: f32 = 1e-4;

    fn approx_eq_vec3(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < EPSILON
    }

    #[test]
    fn character_rides_translating_platform() {
        let mut frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        let mut controller = CharacterController::new();
        let mut position = Vec3::new(0.0, 1.0, 0.0);

        controller.on_ground = true;
        controller.velocity = Vec3::new(5.0, 0.0, 0.0);

        let input = CharacterInput::new();
        let contact = ContactState::grounded();

        for _ in 0..60 {
            frame.integrate(DT);
            let frame_motion = FrameMotion::from_velocity(frame.linear_velocity);
            let output =
                controller.update_in_frame(position, &input, &contact, GRAVITY, &frame_motion, DT);
            position = output.position;
        }

        assert!(position.x > 4.0);
    }

    #[test]
    fn character_jumps_off_moving_platform() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let mut controller = CharacterController::new();
        controller.on_ground = true;
        controller.velocity = Vec3::new(10.0, 0.0, 0.0);

        let jump_input = CharacterInput::new().with_jump(true);
        let contact = ContactState::grounded();
        let frame_motion = FrameMotion::from_velocity(frame.linear_velocity);

        let output = controller.update_in_frame(
            Vec3::new(0.0, 1.0, 0.0),
            &jump_input,
            &contact,
            GRAVITY,
            &frame_motion,
            DT,
        );

        assert!(output.jumped());
        assert!(output.velocity.x > 9.0);
        assert!(output.velocity.y > 0.0);
    }

    #[test]
    fn attachment_tracks_position_on_translating_frame() {
        let mut frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        let world_pos = Vec3::new(0.0, 1.0, 0.0);
        let mut attachment = FrameAttachment::attach(&frame, world_pos, frame.linear_velocity);

        for _ in 0..60 {
            frame.integrate(DT);
            attachment.update(DT);
        }

        let final_world_pos = attachment.world_position(&frame);
        assert!(final_world_pos.x > 4.0);
    }

    #[test]
    fn attachment_tracks_position_on_rotating_frame() {
        let mut frame =
            ReferenceFrame::rotating(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, PI / 2.0, 0.0));
        let world_pos = Vec3::new(2.0, 0.0, 0.0);
        let world_vel = frame.velocity_at_point(world_pos);
        let mut attachment = FrameAttachment::attach(&frame, world_pos, world_vel);

        frame.integrate(1.0);
        attachment.update(1.0);

        let final_world_pos = attachment.world_position(&frame);
        assert!(approx_eq_vec3(final_world_pos, Vec3::new(0.0, 0.0, -2.0)));
    }

    #[test]
    fn detach_inherits_frame_velocity() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let world_pos = Vec3::new(0.0, 1.0, 0.0);
        let world_vel = frame.velocity_at_point(world_pos);
        let attachment = FrameAttachment::attach(&frame, world_pos, world_vel);

        let result = attachment.detach(&frame);

        assert!(approx_eq_vec3(
            result.world_velocity,
            Vec3::new(10.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn detach_from_rotating_frame_inherits_tangential_velocity() {
        let frame = ReferenceFrame::rotating(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, 1.0, 0.0));
        let world_pos = Vec3::new(5.0, 0.0, 0.0);
        let world_vel = frame.velocity_at_point(world_pos);
        let attachment = FrameAttachment::attach(&frame, world_pos, world_vel);

        let result = attachment.detach(&frame);

        assert!(approx_eq_vec3(
            result.world_velocity,
            Vec3::new(0.0, 0.0, -5.0)
        ));
    }

    #[test]
    fn nested_frames_compose_correctly() {
        let mut resolver = FrameResolver::new();

        let parent = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        resolver.insert(1, parent);

        let child =
            ReferenceFrame::with_velocity(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, 0.0, 3.0))
                .with_parent(1);
        resolver.insert(2, child);

        let result = resolver.resolve(2).unwrap();

        assert!(approx_eq_vec3(result.origin, Vec3::new(0.0, 5.0, 0.0)));
        assert!(approx_eq_vec3(
            result.linear_velocity,
            Vec3::new(10.0, 0.0, 3.0)
        ));
    }

    #[test]
    fn character_on_nested_frames() {
        let mut resolver = FrameResolver::new();

        let ship = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0));
        resolver.insert(1, ship);

        let elevator =
            ReferenceFrame::with_velocity(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0))
                .with_parent(1);
        resolver.insert(2, elevator);

        let result = resolver.resolve(2).unwrap();

        let mut controller = CharacterController::new();
        controller.on_ground = true;
        controller.velocity = result.linear_velocity;

        let input = CharacterInput::new();
        let contact = ContactState::grounded();
        let frame_motion = FrameMotion::from_velocity(result.linear_velocity);

        let output = controller.update_in_frame(
            Vec3::new(0.0, 1.0, 0.0),
            &input,
            &contact,
            GRAVITY,
            &frame_motion,
            DT,
        );

        assert!(output.velocity.x > 19.0);
        assert!(output.velocity.y > 1.0);
    }

    #[test]
    fn static_frame_behaves_like_no_frame() {
        let mut controller1 = CharacterController::new();
        let mut controller2 = CharacterController::new();

        let input = CharacterInput::horizontal(0.0, 1.0);
        let contact = ContactState::grounded();
        let static_frame = FrameMotion::default();

        controller1.on_ground = true;
        controller2.on_ground = true;

        for _ in 0..30 {
            let _ = controller1.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);
            let _ = controller2.update_in_frame(
                Vec3::ZERO,
                &input,
                &contact,
                GRAVITY,
                &static_frame,
                DT,
            );
        }

        assert!((controller1.velocity - controller2.velocity).length() < 0.01);
    }

    #[test]
    fn accelerating_frame_adds_pseudo_force() {
        let frame =
            ReferenceFrame::with_linear_motion(Vec3::ZERO, Vec3::ZERO, Vec3::new(0.0, 5.0, 0.0));

        let pseudo = frame.pseudo_force_at_point(Vec3::ZERO, 1.0);

        assert!(approx_eq_vec3(pseudo, Vec3::new(0.0, -5.0, 0.0)));
    }

    #[test]
    fn rotating_frame_centrifugal_force() {
        let frame = ReferenceFrame::rotating(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, 1.0, 0.0));
        let point = Vec3::new(10.0, 0.0, 0.0);

        let pseudo = frame.pseudo_force_at_point(point, 1.0);

        assert!(pseudo.x > 0.0);
        assert!(pseudo.y.abs() < EPSILON);
    }

    #[test]
    fn frame_integration_deterministic() {
        let mut frame1 = ReferenceFrame::with_linear_motion(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 0.0, 0.0),
        );
        let mut frame2 = frame1.clone();

        for _ in 0..100 {
            frame1.integrate(DT);
        }

        for _ in 0..100 {
            frame2.integrate(DT);
        }

        assert!(approx_eq_vec3(frame1.origin, frame2.origin));
        assert!(approx_eq_vec3(
            frame1.linear_velocity,
            frame2.linear_velocity
        ));
    }

    #[test]
    fn local_movement_stays_local() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0));
        let mut attachment = FrameAttachment::attach(&frame, Vec3::ZERO, frame.linear_velocity);

        attachment.apply_local_movement(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0));

        assert!(approx_eq_vec3(
            attachment.local_position,
            Vec3::new(1.0, 0.0, 0.0)
        ));
        assert!(approx_eq_vec3(
            attachment.local_velocity,
            Vec3::new(2.0, 0.0, 0.0)
        ));

        let world_vel = attachment.world_velocity(&frame);
        assert!(approx_eq_vec3(world_vel, Vec3::new(102.0, 0.0, 0.0)));
    }

    #[test]
    fn walking_on_platform_with_surface_velocity_in_contact() {
        let mut controller = CharacterController::new();
        controller.on_ground = true;
        controller.velocity = Vec3::new(5.0, 0.0, 0.0);

        let input = CharacterInput::new();
        let contact = ContactState::grounded_on_frame(1, Vec3::new(5.0, 0.0, 0.0));

        let frame_motion = FrameMotion::from_velocity(contact.surface_velocity);
        let output = controller.update_in_frame(
            Vec3::new(0.0, 1.0, 0.0),
            &input,
            &contact,
            GRAVITY,
            &frame_motion,
            DT,
        );

        assert!(output.on_ground);
        assert!(output.velocity.x > 4.0);
    }
}
