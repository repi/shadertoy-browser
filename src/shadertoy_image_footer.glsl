layout(location = 0) in vec2 _fragCoord;
layout(location = 0) out vec4 _fragColor;

void main() 
{ 
	mainImage(_fragColor, _fragCoord); 
}
