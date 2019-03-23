use crate::color::Color;
use crate::light::PointLight;
use crate::linear::{Matrix4, Point};
use crate::materials::Material;
use crate::rays::{Intersection, Precomputation, Ray};
use crate::shapes::{Shape, Sphere};

pub const MAX_REFLECTIONS: usize = 4;

pub struct World {
    pub objects: Vec<Box<Shape>>,
    pub light_source: PointLight,
}

impl World {
    pub fn default_sphere1() -> Sphere {
        let mut s1 = Sphere::new();
        s1.set_material(Material {
            color: Color::new(0.8, 1.0, 0.6),
            diffuse: 0.7,
            specular: 0.2,
            ..Material::default()
        });
        s1
    }

    pub fn default_sphere2() -> Sphere {
        let mut s2 = Sphere::new();
        s2.set_transform(Matrix4::scaling(0.5, 0.5, 0.5));
        s2
    }

    pub fn default() -> Self {
        let light = PointLight::new(Point::new(-10.0, 10.0, -10.0), Color::white());
        let s1: Box<Shape> = Box::from(Self::default_sphere1());
        let s2: Box<Shape> = Box::from(Self::default_sphere2());

        World {
            objects: vec![s1, s2],
            light_source: light,
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Vec<Intersection> {
        let mut out: Vec<Intersection> = vec![];
        for object in &self.objects {
            out.append(&mut object.intersect(&ray))
        }
        out.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    pub fn shade(&self, c: &Precomputation, remaining: usize) -> Color {
        let surface = c.object.material().lighting(
            c.object,
            &self.light_source,
            &c.over_point,
            &c.eye,
            &c.normal,
            self.is_shadowed(&c.over_point),
        );
        let reflected = self.reflected_color(&c, remaining);
        let refracted = self.refracted_color(&c, remaining);
        surface + reflected + refracted
    }

    pub fn color_at(&self, r: &Ray, remaining: usize) -> Color {
        let intersections = self.intersect(&r);
        if let Some(hit) = Intersection::hit(&intersections) {
            self.shade(&hit.precompute(&r, &intersections), remaining)
        } else {
            Color::black()
        }
    }

    pub fn reflected_color(&self, comps: &Precomputation, remaining: usize) -> Color {
        let mat = comps.object.material();
        if remaining <= 0 || mat.reflective == 0.0 {
            Color::black()
        } else {
            let reflect_ray = Ray::new(comps.over_point, comps.reflect);
            let color = self.color_at(&reflect_ray, remaining - 1);
            color * mat.reflective
        }
    }

    pub fn refracted_color(&self, comps: &Precomputation, remaining: usize) -> Color {
        if remaining == 0 || comps.object.material().transparency == 0.0 {
            Color::black()
        } else {
            let n_ratio = comps.n1 / comps.n2;
            let cos_i = comps.eye.dot(&comps.normal);
            let sin2_t = n_ratio.powf(2.0) * (1.0 - cos_i.powf(2.0));
            if sin2_t > 1.0 {
                Color::black()
            } else {
              let cos_t = (1.0 - sin2_t).sqrt();
              let direction = (comps.normal * ((n_ratio * cos_i) - cos_t)) - (comps.eye * n_ratio);
              let refract_ray = Ray::new(comps.under_point, direction);
              self.color_at(&refract_ray, remaining - 1) * comps.object.material().transparency
            }
        }
    }

    pub fn is_shadowed(&self, p: &Point) -> bool {
        let v = self.light_source.position - *p;
        let distance = v.magnitude();
        let direction = v.normalize();

        let r = Ray::new(*p, direction);
        let mut intersections = self.intersect(&r);

        match Intersection::hit(&mut intersections) {
            Some(h) if h.t < distance => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::light::PointLight;
    use crate::linear::{Matrix4, Point, Vector, EPSILON};
    use crate::materials::Material;
    use crate::shapes::Plane;

    #[test]
    fn default_world() {
        let light = PointLight::new(Point::new(-10.0, 10.0, -10.0), Color::white());
        let mut s1 = Sphere::new();
        s1.material = Material {
            color: Color::new(0.8, 1.0, 0.6),
            diffuse: 0.7,
            specular: 0.2,
            ..s1.material
        };
        let mut s2 = Sphere::new();
        s2.set_transform(Matrix4::scaling(0.5, 0.5, 0.5));

        let w = World::default();
        assert_eq!(w.light_source, light);
    }

    #[test]
    fn intersect_world() {
        let w = World::default();
        let r = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let hits = w.intersect(&r).iter().map(|x| x.t).collect::<Vec<f32>>();
        assert_eq!(hits, vec!(4.0, 4.5, 5.5, 6.0));
    }

    #[test]
    fn precomputing_state_of_intersection() {
        let r = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let s = Sphere::new();
        let i = Intersection { t: 4.0, object: &s };
        let is = [i];
        let comps = i.precompute(&r, &is);
        assert_eq!(comps.point, Point::new(0.0, 0.0, -1.0));
        assert_eq!(comps.eye, Vector::new(0.0, 0.0, -1.0));
        assert_eq!(comps.normal, Vector::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn shading_intersection() {
        let w = World::default();
        let r = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let shape: &Box<Shape> = w.objects.first().unwrap();
        let i = Intersection {
            t: 4.0,
            object: &**shape,
        };
        let is = [i];
        let comps = i.precompute(&r, &is);
        assert_eq!(w.shade(&comps, 1), Color::new(0.38066, 0.47583, 0.2855));
    }

    #[test]
    fn shading_intersection_from_inside() {
        let mut w = World::default();
        w.light_source = PointLight::new(Point::new(0.0, 0.25, 0.0), Color::white());
        let r = Ray::new(Point::origin(), Vector::new(0.0, 0.0, 1.0));
        let shape: &Box<Shape> = w.objects.last().unwrap();
        let i = Intersection {
            t: 0.5,
            object: &**shape,
        };
        let is = [i];
        let comps = i.precompute(&r, &is);
        assert_eq!(w.shade(&comps, 1), Color::new(0.90498, 0.90498, 0.90498));
    }

    #[test]
    fn color_at_miss() {
        let w = World::default();
        let r = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 1.0, 0.0));
        let c = w.color_at(&r, 1);
        assert_eq!(c, Color::black());
    }

    #[test]
    fn color_at_hit() {
        let w = World::default();
        let r = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let c = w.color_at(&r, 1);
        assert_eq!(c, Color::new(0.38066, 0.47582, 0.2855));
    }

    #[test]
    fn no_shadow_when_nothing_is_collinear_with_point_and_light() {
        let w = World::default();
        let p = Point::new(0.0, 10.0, 0.0);
        assert_eq!(w.is_shadowed(&p), false);
    }

    #[test]
    fn shadow_when_object_is_between_light_and_point() {
        let w = World::default();
        let p = Point::new(10.0, -10.0, 10.0);
        assert_eq!(w.is_shadowed(&p), true);
    }

    #[test]
    fn no_shadow_when_object_is_behind_the_light() {
        let w = World::default();
        let p = Point::new(-20.0, 20.0, -20.0);
        assert_eq!(w.is_shadowed(&p), false);
    }

    #[test]
    fn no_shadow_when_object_is_behind_the_point() {
        let w = World::default();
        let p = Point::new(-2.0, 2.0, -2.0);
        assert_eq!(w.is_shadowed(&p), false);
    }

    #[test]
    fn hit_should_offset_the_point() {
        let r = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let mut s = Sphere::new();
        s.set_transform(Matrix4::translation(0.0, 0.0, 1.0));
        let i = Intersection { t: 5.0, object: &s };
        let mut is = [i];
        let comps = i.precompute(&r, &mut is);
        assert!(comps.over_point.z < -EPSILON / 2.0);
        assert!(comps.point.z > comps.over_point.z);
    }

    use std::rc::Rc;
    use std::cell::RefCell;
    use crate::patterns::TestPattern;

    fn update_world(w: &Rc<RefCell<World>>, f: fn(&mut World) -> ()) {
        f(&mut w.borrow_mut());
    }

    #[test]
    fn reflected_color_for_nonreflective_material() {
        let w = Rc::new(RefCell::new(World::default()));
        let r = Ray::new(Point::origin(), Vector::new(0.0, 0.0, 1.0));
        let i: Intersection;

        update_world(&w, |world: &mut World| {
            let s = world.objects.last_mut().unwrap();
            s.set_material(Material {
                ambient: 1.0,
                ..World::default_sphere2().material
            });
        });

        let world = w.borrow();
        let s = world.objects.last().unwrap();
        i = Intersection { t: 1.0, object: &**s };
        let is = [i];
        let comps = i.precompute(&r, &is);
        let color = world.reflected_color(&comps, 1);
        assert_eq!(color, Color::black());
    }

    #[test]
    fn reflected_color_for_reflective_material() {
        let w = Rc::new(RefCell::new(World::default()));

        update_world(&w, |world: &mut World| {
            let mut p = Plane::new();
            p.material.reflective = 0.5;
            p.set_transform(Matrix4::translation(0.0, -1.0, 0.0));
            let s = world.objects.last_mut().unwrap();
            s.set_material(Material {
                ambient: 1.0,
                ..World::default_sphere2().material
            });
            world.objects.push(Box::from(p));
        });

        let world = w.borrow();
        let plane = world.objects.last().unwrap();
        let r = Ray::new(Point::new(0.0, 0.0, -3.0), Vector::new(0.0, -2_f32.sqrt()/2.0, 2_f32.sqrt()/2.0));
        let i = Intersection { t: 2_f32.sqrt(), object: &**plane };
        let is = [i];
        let comps = i.precompute(&r, &is);
        let color = world.reflected_color(&comps, 1);
        assert_eq!(color, Color::new(0.19032, 0.2379, 0.14274));
    }

    #[test]
    fn shade_with_a_reflective_material() {
        let w = Rc::new(RefCell::new(World::default()));

        update_world(&w, |world: &mut World| {
            let mut p = Plane::new();
            p.material.reflective = 0.5;
            p.set_transform(Matrix4::translation(0.0, -1.0, 0.0));
            world.objects.push(Box::from(p));
        });

        let world = w.borrow();
        let plane = world.objects.last().unwrap();
        let r = Ray::new(Point::new(0.0, 0.0, -3.0), Vector::new(0.0, -2_f32.sqrt()/2.0, 2_f32.sqrt()/2.0));
        let i = Intersection { t: 2_f32.sqrt(), object: &**plane };
        let is = [i];
        let comps = i.precompute(&r, &is);
        let color = world.shade(&comps, 1);
        assert_eq!(color, Color::new(0.87677, 0.92436, 0.82918));
    }

    #[test]
    fn color_at_with_mutually_recursive_surfaces() {
        let w = Rc::new(RefCell::new(World::default()));

        update_world(&w, |world: &mut World| {
            world.light_source = PointLight::new(Point::origin(), Color::white());
            let mut lower = Plane::new();
            lower.material.reflective = 1.0;
            lower.set_transform(Matrix4::translation(0.0, -1.0, 0.0));
            let mut upper = Plane::new();
            upper.material.reflective = 1.0;
            upper.set_transform(Matrix4::translation(0.0, 1.0, 0.0));
            world.objects.push(Box::from(lower));
            world.objects.push(Box::from(upper));
        });

        let world = w.borrow();
        let r = Ray::new(Point::origin(), Vector::new(0.0, 1.0, 0.0));
        let color = world.color_at(&r, MAX_REFLECTIONS);
        assert_eq!(color, Color::new(1.9, 1.9, 1.9));
    }

    #[test]
    fn refracted_color_with_opaque_surface() {
        let world = World::default();
        let shape = world.objects.first().unwrap();
        let r = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let is = vec![
            Intersection { t: 4.0, object: &**shape },
            Intersection { t: 6.0, object: &**shape }
        ];
        let i = is[0];
        let comps = i.precompute(&r, &is);
        let color = world.refracted_color(&comps, MAX_REFLECTIONS);
        assert_eq!(color, Color::black());
    }

    #[test]
    fn refracted_color_at_maximum_recursive_depth() {
        let w = Rc::new(RefCell::new(World::default()));

        update_world(&w, |world: &mut World| {
            let first = world.objects.first_mut().unwrap();
            first.set_material(Material {
                transparency: 1.0,
                refractive_index: 1.5,
                ..World::default_sphere1().material
            });
        });

        let world = w.borrow();
        let r = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let shape = world.objects.first().unwrap();
        let is = vec![
            Intersection { t: 4.0, object: &**shape },
            Intersection { t: 6.0, object: &**shape }
        ];
        let i = is[0];
        let comps = i.precompute(&r, &is);
        let color = world.refracted_color(&comps, 0);
        assert_eq!(color, Color::black());
    }

    #[test]
    fn refracted_color_under_total_internal_reflection() {
        let w = Rc::new(RefCell::new(World::default()));

        update_world(&w, |world: &mut World| {
            let first = world.objects.first_mut().unwrap();
            first.set_material(Material {
                transparency: 1.0,
                refractive_index: 1.5,
                ..World::default_sphere1().material
            });
        });

        let world = w.borrow();
        let r = Ray::new(Point::new(0.0, 0.0, 2_f32.sqrt()), Vector::new(0.0, 1.0, 0.0));
        let shape = world.objects.first().unwrap();
        let is = vec![
            Intersection { t: -2_f32.sqrt(), object: &**shape },
            Intersection { t: 2_f32.sqrt(), object: &**shape }
        ];
        let i = is[1];
        let comps = i.precompute(&r, &is);
        let color = world.refracted_color(&comps, MAX_REFLECTIONS);
        assert_eq!(color, Color::black());
    }

    #[test]
    fn refracted_color_with_refracted_ray() {
        let w = Rc::new(RefCell::new(World::default()));

        update_world(&w, |world: &mut World| {
            let first = world.objects.first_mut().unwrap();
            first.set_material(Material {
                ambient: 1.0,
                pattern: Some(Box::from(TestPattern::new())),
                ..World::default_sphere1().material
            });
            let second = world.objects.last_mut().unwrap();
            second.set_material(Material {
                transparency: 1.0,
                refractive_index: 1.5,
                ..World::default_sphere2().material
            });
        });

        let world = w.borrow();
        let r = Ray::new(Point::new(0.0, 0.0, 0.1), Vector::new(0.0, 1.0, 0.0));
        let a = world.objects.first().unwrap();
        let b = world.objects.last().unwrap();
        let is = vec![
            Intersection { t: -0.9899, object: &**a },
            Intersection { t: -0.4899, object: &**b },
            Intersection { t: 0.4899, object: &**b },
            Intersection { t: 0.9899, object: &**a }
        ];
        let i = is[2];
        let comps = i.precompute(&r, &is);
        let color = world.refracted_color(&comps, MAX_REFLECTIONS);
        assert_eq!(color, Color::new(0.0, 0.99888, 0.04725));
    }

    #[test]
    fn shade_with_transparent_material() {
        let w = Rc::new(RefCell::new(World::default()));

        update_world(&w, |world: &mut World| {
            let mut floor = Plane::new();
            floor.set_transform(Matrix4::translation(0.0, -1.0, 0.0));
            floor.material.transparency = 0.5;
            floor.material.refractive_index = 1.5;

            let mut ball = Sphere::new();
            ball.material.color = Color::new(1.0, 0.0, 0.0);
            ball.material.ambient = 0.5;
            ball.set_transform(Matrix4::translation(0.0, -3.5, -0.5));

            world.objects.push(Box::from(ball));
            world.objects.push(Box::from(floor));
        });

        let world = w.borrow();
        let r = Ray::new(Point::new(0.0, 0.0, -3.0), Vector::new(0.0, -2_f32.sqrt()/2.0, 2_f32.sqrt()/2.0));
        let floor = world.objects.last().unwrap();
        let is = vec![
            Intersection { t: 2_f32.sqrt(), object: &**floor },
        ];
        let i = is[0];
        let comps = i.precompute(&r, &is);
        let color = world.shade(&comps, MAX_REFLECTIONS);
        assert_eq!(color, Color::new(0.93642, 0.68642, 0.68642));
    }
}
