use nsi;
use polyhedron_ops as p_ops;

use std::{env, path::PathBuf};

fn nsi_camera(c: &nsi::Context, name: &str, camera_xform: &[f64; 16]) {
    // Setup a camera transform.
    c.create("camera_xform", nsi::NodeType::Transform, &[]);
    c.connect("camera_xform", "", ".root", "objects", &[]);
    c.set_attribute(
        "camera_xform",
        &[nsi::double_matrix!(
            "transformationmatrix",
            &[1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 5., 1.,]
        )],
    );

    // Setup a camera.
    c.create("camera", nsi::NodeType::PerspectiveCamera, &[]);
    c.connect("camera", "", "camera_xform", "objects", &[]);
    c.set_attribute("camera", &[nsi::float!("fov", 35.)]);

    // Setup a screen.
    c.create("screen", nsi::NodeType::Screen, &[]);
    c.connect("screen", "", "camera", "screens", &[]);
    c.set_attribute(
        "screen",
        &[
            nsi::integers!("resolution", &[256, 256]).array_len(2),
            nsi::integer!("oversampling", 16),
        ],
    );

    c.set_attribute(
        ".global",
        &[
            nsi::integer!("renderatlowpriority", 1),
            nsi::string!("bucketorder", "circle"),
            nsi::unsigned!("quality.shadingsamples", 4),
            nsi::integer!("maximumraydepth.reflection", 6),
        ],
    );

    // RGB layer.
    c.create("beauty", nsi::NodeType::OutputLayer, &[]);
    c.set_attribute(
        "beauty",
        &[
            nsi::string!("variablename", "Ci"),
            nsi::integer!("withalpha", 1),
            nsi::string!("scalarformat", "float"),
        ],
    );
    c.connect("beauty", "", "screen", "outputlayers", &[]);

    // Normal layer.
    c.create("albedo", nsi::NodeType::OutputLayer, &[]);
    c.set_attribute(
        "albedo",
        &[
            nsi::string!("variablename", "albedo"),
            nsi::string!("variablesource", "shader"),
            nsi::string!("layertype", "color"),
            nsi::string!("scalarformat", "float"),
            //nsi::string!("filter", "box"),
            //nsi::double!("filterwidth", 1.),
        ],
    );
    c.connect("albedo", "", "screen", "outputlayers", &[]);

    // Normal layer.
    c.create("normal", nsi::NodeType::OutputLayer, &[]);
    c.set_attribute(
        "normal",
        &[
            nsi::string!("variablename", "N.world"),
            nsi::string!("variablesource", "builtin"),
            nsi::string!("layertype", "vector"),
            nsi::string!("scalarformat", "float"),
            //nsi::string!("filter", "box"),
            //nsi::double!("filterwidth", 1.),
        ],
    );
    c.connect("normal", "", "screen", "outputlayers", &[]);

    // Setup an output driver.
    c.create("driver", nsi::NodeType::OutputDriver, &[]);
    c.connect("driver", "", "beauty", "outputdrivers", &[]);
    //c.connect("driver", "", "albedo", "outputdrivers", &[]);
    //c.connect("driver", "", "normal", "outputdrivers", &[]);

    c.set_attribute(
        "driver",
        &[
            nsi::string!("drivername", "r-display"),
            nsi::string!("imagefilename", "test_output.exr"),
            nsi::unsigned!("denoise", 1),
        ],
    );

    c.create("driver2", nsi::NodeType::OutputDriver, &[]);
    //c.connect("driver2", "", "beauty", "outputdrivers", &[]);
    c.connect("driver2", "", "beauty", "outputdrivers", &[]);

    c.set_attribute("driver2", &[nsi::string!("drivername", "idisplay")]);
}

fn nsi_environment(c: &nsi::Context) {
    if let Ok(path) = &env::var("DELIGHT") {
        // Set up an environment light.
        c.create("env_xform", nsi::NodeType::Transform, &[]);
        c.connect("env_xform", "", ".root", "objects", &[]);

        c.create("environment", nsi::NodeType::Environment, &[]);
        c.connect("environment", "", "env_xform", "objects", &[]);

        c.create("env_attrib", nsi::NodeType::Attributes, &[]);
        c.connect("env_attrib", "", "environment", "geometryattributes", &[]);

        c.set_attribute("env_attrib", &[nsi::integer!("visibility.camera", 0)]);

        c.create("env_shader", nsi::NodeType::Shader, &[]);
        c.connect("env_shader", "", "env_attrib", "surfaceshader", &[]);

        // Environment light attributes.
        c.set_attribute(
            "env_shader",
            &[
                nsi::string!(
                    "shaderfilename",
                    PathBuf::from(path)
                        .join("osl")
                        .join("environmentLight")
                        .to_string_lossy()
                        .into_owned()
                ),
                nsi::float!("intensity", 1.),
            ],
        );

        c.set_attribute(
            "env_shader",
            &[nsi::string!("image", "assets/wooden_lounge_2k.tdl")],
        );
    }
}

fn nsi_reflective_ground(c: &nsi::Context, _name: &str) {
    if let Ok(path) = &env::var("DELIGHT") {
        c.create("ground_xform", nsi::NodeType::Transform, &[]);
        c.connect("ground_xform", "", ".root", "objects", &[]);
        c.set_attribute(
            "ground_xform",
            &[nsi::double_matrix!(
                "transformationmatrix",
                &[1., 0., 0., 0., 0., 0., -1., 0., 0., 1., 0., 0., 0., -1., 0., 1.,]
            )],
        );

        c.create("ground", nsi::NodeType::Plane, &[]);
        c.connect("ground", "", "ground_xform", "objects", &[]);

        c.create("ground_attrib", nsi::NodeType::Attributes, &[]);
        c.connect("ground_attrib", "", "ground", "geometryattributes", &[]);

        // Ground shader.
        c.create("ground_shader", nsi::NodeType::Shader, &[]);
        c.connect("ground_shader", "", "ground_attrib", "surfaceshader", &[]);

        c.set_attribute(
            "ground_shader",
            &[
                nsi::string!(
                    "shaderfilename",
                    //"osl/dlPrincipled"
                    PathBuf::from(path)
                        .join("osl")
                        .join("dlPrincipled")
                        .to_string_lossy()
                        .into_owned()
                ),
                nsi::color!("i_color", &[0.001, 0.001, 0.001]),
                //nsi::arg!("coating_thickness", &0.1f32),
                nsi::float!("roughness", 0.1),
                nsi::float!("specular_level", 1.),
                nsi::float!("metallic", 1.),
                nsi::float!("anisotropy", 0.),
                nsi::float!("sss_weight", 0.),
                nsi::color!("sss_color", &[0.5, 0.5, 0.5]),
                nsi::float!("sss_scale", 0.),
                nsi::color!("incandescence", &[0., 0., 0.]),
                nsi::float!("incandescence_intensity", 0.),
                //nsi::color!("incandescence_multiplier", &[1.0f32, 1.0, 1.0]),
            ],
        );
    }
}

fn nsi_material(c: &nsi::Context, name: &str) {
    if let Ok(path) = &env::var("DELIGHT") {
        // Particle attributes.
        let attribute_name = format!("{}_attrib", name);
        c.create(attribute_name.clone(), nsi::NodeType::Attributes, &[]);
        c.connect(attribute_name.clone(), "", name, "geometryattributes", &[]);

        // Metal shader.
        let shader_name = format!("{}_shader", name);
        c.create(shader_name.clone(), nsi::NodeType::Shader, &[]);
        c.connect(
            shader_name.clone(),
            "",
            attribute_name,
            "surfaceshader",
            &[],
        );

        c.set_attribute(
            shader_name,
            &[
                nsi::string!(
                    "shaderfilename",
                    //"osl/dlPrincipled"
                    PathBuf::from(path)
                        .join("osl")
                        .join("dlPrincipled")
                        .to_string_lossy()
                        .into_owned()
                ),
                nsi::color!("i_color", &[1.0f32, 0.6, 0.3]),
                //nsi::arg!("coating_thickness", &0.1f32),
                nsi::float!("roughness", 0.3f32),
                nsi::float!("specular_level", 0.5f32),
                nsi::float!("metallic", 1.0f32),
                nsi::float!("anisotropy", 0.0f32),
                nsi::float!("sss_weight", 0.0f32),
                nsi::color!("sss_color", &[0.5f32, 0.5, 0.5]),
                nsi::float!("sss_scale", 0.0f32),
                nsi::color!("incandescence", &[0.0f32, 0.0, 0.0]),
                nsi::float!("incandescence_intensity", 0.0f32),
                //nsi::color!("incandescence_multiplier", &[1.0f32, 1.0, 1.0]),
            ],
        );
    }
}

pub fn nsi_render(
    polyhedron: &p_ops::Polyhedron,
    camera_xform: &[f64; 16],
    name: &str,
    cloud_render: bool,
) {
    let ctx = {
        if cloud_render {
            nsi::Context::new(&[
                nsi::integer!("cloud", 1),
                nsi::string!("software", "HOUDINI"),
            ])
        } else {
            nsi::Context::new(&[])
        }
    }
    .expect("Could not create NSI rendering context.");

    nsi_camera(&ctx, name, camera_xform);

    nsi_environment(&ctx);

    let name = polyhedron.to_nsi(&ctx);

    nsi_material(&ctx, &name);

    nsi_reflective_ground(&ctx, &name);

    // And now, render it!
    ctx.render_control(&[nsi::string!("action", "start")]);
    ctx.render_control(&[nsi::string!("action", "wait")]);
}

fn main() {
    let mut polyhedron = p_ops::Polyhedron::tetrahedron();
    polyhedron.meta(true);
    polyhedron.normalize();
    polyhedron.gyro(1. / 3., 0.1, true);
    polyhedron.normalize();
    polyhedron.kis(-0.2, None, true, true);
    polyhedron.normalize();

    nsi_render(&polyhedron, &[0.0f64; 16], "foo", false);
}
