&NewLine;
## Mathematical Background

Understanding the mathematical basis is essential for grasping how angle distortion is calculated.

Given a triangle with vertices $ A $, $ B $, and $ C $, the angle at vertex $ C $ is calculated using:

$$
\theta_C = \arccos \left( \frac{ (\mathbf{A} - \mathbf{C}) \cdot (\mathbf{B} - \mathbf{C}) }{ \| \mathbf{A} - \mathbf{C} \| \, \| \mathbf{B} - \mathbf{C} \| } \right)
$$

**Where:**

- $ \mathbf{A}, \mathbf{B}, \mathbf{C} $ are the position vectors of the triangle's vertices.
- $ \cdot $ denotes the dot product of two vectors.
- $ \| \cdot \| $ denotes the Euclidean norm (magnitude) of a vector.
- $ \arccos $ is the inverse cosine function, returning the angle in radians.
