import { Course, CourseMap, CourseProgress } from "../bindings";
import { sortCourseMaps } from "../util";

export function buildCourseMapListing(
	courseMapping: Map<string, [Course, CourseProgress]>,
	courseMaps: [CourseMap, string][],
	contentViewer: HTMLElement,
	styleContainer: HTMLStyleElement,
): DocumentFragment {
	const fragment = document.createDocumentFragment();

	const header = document.createElement("h2");
	header.innerText = "🗺️ Course Maps";

	fragment.appendChild(header);

	sortCourseMaps(courseMaps);

	const list = document.createElement("ul");

	const courseMapMap: Map<string, [CourseMap, string]> = new Map();

	for (const [courseMap, svg] of courseMaps) {
		const element = document.createElement("li");
		element.id = "map-" + courseMap.uuid;

		const label = document.createElement("a");
		label.setAttribute("tabindex", "0");
		label.setAttribute("role", "button");
		label.innerText = courseMap.title;

		element.appendChild(label);

		courseMapMap.set(courseMap.uuid, [courseMap, svg]);

		list.appendChild(element);
	}

	const clickListener = (event: Event) => {
		const target = event.target as HTMLElement;

		if (target.tagName == "A" && target.parentElement) {
			const identifier = target.parentElement.id.substring(4);
			const courseMap = courseMapMap.get(identifier);

			if (courseMap) {
				contentViewer.innerHTML = "";
				contentViewer.appendChild(
					buildCourseMapInfo(courseMap[0], courseMap[1], courseMapping),
				);

				styleContainer.innerHTML =
					"#map-" + courseMap[0].uuid + " {font-weight: bold}";
			}
		}
	};

	list.addEventListener("click", clickListener);
	list.addEventListener("keydown", (event) => {
		if (event.code == "Enter") {
			clickListener(event);
		}
	});

	fragment.appendChild(list);

	if (courseMaps.length != 0) {
		fragment.appendChild(document.createElement("br"));
	}

	return fragment;
}

function buildCourseMapInfo(
	courseMap: CourseMap,
	svg: string,
	_courseMapping: Map<string, [Course, CourseProgress]>,
) {
	const root = document.createDocumentFragment();

	const title = document.createElement("h2");
	title.innerText = "🗺️ " + courseMap.title;
	root.appendChild(title);

	if (courseMap.description) {
		const description = document.createElement("p");
		description.innerText = courseMap.description;
		root.appendChild(description);
	}

	const image = document.createElement("div");
	image.classList.add("image");
	image.innerHTML = svg;

	const svgElement = image.getElementsByTagName("svg")[0];

	console.log(svgElement);

	//modifyCourseMapSvg(svgElement);

	root.appendChild(image);

	return root;
}

// Next plans:
// - Display if a ✔️ next to completed Courses within a Map
// - Allow clicking on a Course to view it's details
// - Better handle CourseMap descriptions
