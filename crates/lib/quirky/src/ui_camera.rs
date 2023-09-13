use glam::{vec3, UVec2, Vec4};

#[derive(bytemuck::Zeroable, bytemuck::Pod, Copy, Clone)]
#[repr(C)]
pub struct UiCameraUniform {
    pub transform: [[f32; 4]; 4],
}

#[derive(Default)]
pub struct UiCamera2D {
    transform: glam::Mat4,
    screen_resolution: UVec2
}

impl UiCamera2D {
    pub fn create_camera_uniform(&self) -> UiCameraUniform {
        UiCameraUniform {
            transform: self.transform.to_cols_array_2d()
        }
    }

    pub fn resize_viewport(&mut self, size: UVec2) {
        self.screen_resolution = size;
        let camera_origin_translate = glam::Mat4::from_translation(vec3(-1.0, 1.0, 0.0));

        let camera_coordinate_transform = glam::mat4(
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, -1.0, 0.0, 0.0),
            Vec4::new(0.0, -1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        let ndc_scale = glam::mat4(
            Vec4::new(2.0 / size.x as f32, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 2.0 / size.y as f32, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        self.transform = camera_origin_translate * ndc_scale * camera_coordinate_transform;
    }

    pub fn viewport_size(&self) -> UVec2 {
        self.screen_resolution
    }

    #[cfg(test)]
    pub fn transform(&self) -> &glam::Mat4 {
        &self.transform
    }
}

#[cfg(test)]
mod test {
    use crate::assert_f32_eq;
    use crate::ui_camera::UiCamera2D;
    use glam::{vec4, UVec2, Vec4};

    #[test]
    fn test_camera_transform() {
        let mut cam = UiCamera2D::default();

        cam.resize_viewport(UVec2::new(800, 600));

        let points_to_test = vec![
            (vec4(0.0, 0.0, 0.0, 1.0), vec4(-1.0, 1.0, 0.0, 0.0)),
            (vec4(800.0, 600.0, 0.0, 1.0), vec4(1.0, -1.0, 0.0, 0.0)),
            (vec4(0.0, 600.0, 0.0, 1.0), vec4(-1.0, -1.0, 0.0, 0.0)),
            (vec4(800.0, 0.0, 0.0, 1.0), vec4(1.0, 1.0, 0.0, 0.0)),
        ];

        points_to_test.iter().for_each(|p| {
            check_transform(p.0, p.1, &cam);
        });
    }

    fn check_transform(pixel_coord: Vec4, ndc_space: Vec4, cam: &UiCamera2D) {
        let pixel_ndc = cam.transform().mul_vec4(pixel_coord);

        assert_f32_eq!(pixel_ndc.x, ndc_space.x, "ndc x");
        assert_f32_eq!(pixel_ndc.y, ndc_space.y, "ndc y");

        let reverse_transform = cam.transform().inverse();
        let pixel_coord_rev = reverse_transform.mul_vec4(pixel_ndc);

        assert_f32_eq!(pixel_coord_rev.x, pixel_coord.x, "rev px x");
        assert_f32_eq!(pixel_coord_rev.y, pixel_coord.y, "rev px y");
    }
}
