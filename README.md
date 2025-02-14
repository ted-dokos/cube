A small rendering engine built in Rust. I built this to learn more about shaders and Rust's wgpu library.

The engine has separate threads for the rendering pipeline and the input processing. I enjoyed how easy it was to set that up in Rust.

Some constructions:
* An 'aerogel' effect, created using ray-marching technique:

  ![video](https://github.com/user-attachments/assets/57cc1643-8679-4d2a-88c2-c1315686f189)

* Surface normal transitions, allowing objects to transition between flat and smooth shading. The shading and lighting is a basic implementation of Phong shading.

  https://github.com/user-attachments/assets/bc2c4d43-e88a-474d-9fdf-52df5d8b97c5
  ![puffy](https://github.com/user-attachments/assets/0d733354-49b2-460b-a5bf-5664c257d19d)
  ![dimpled](https://github.com/user-attachments/assets/b68738a3-99a3-4ae1-bc08-bf99087a7d38)
