import { FitAddon } from "@xterm/addon-fit";
import { useEffect, useMemo, useState } from "react";
import { useXTerm } from "react-xtermjs";
import * as recipesData from "../../../assets/recipes.json";
import * as z from "zod";

const recipeSchema = z.object({
  description: z.string(),
  steps: z.array(
    z.object({
      description: z.string(),
      key: z.string(),
      term_output: z.string(),
    })
  ),
});

type Recipe = z.infer<typeof recipeSchema>;
const recipes = z.array(recipeSchema).parse(recipesData.recipes_output);

export const Recipes = () => {
  return (
    <div style={{ display: "grid", gap: 16 }}>
      {recipes.map((recipe, index) => (
        <Recipe key={index} recipe={recipe} />
      ))}
    </div>
  );
};

export const Recipe = (props: { recipe: Recipe }) => {
  const { instance, ref } = useXTerm();
  const fitAddon = new FitAddon();
  const [stepIndex, setStepIndex] = useState(0);
  useEffect(() => {
    const step = props.recipe.steps[stepIndex];
    // Load the fit addon
    instance?.loadAddon(fitAddon);

    const handleResize = () => fitAddon.fit();

    instance?.clear();
    instance?.write(step.term_output);

    // Handle resize event
    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, [ref, instance, stepIndex]);

  return (
    <div style={{ display: "grid" }}>
      <div>{props.recipe.description}</div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr" }}>
        <div ref={ref} style={{ height: "100%", width: "100%" }} />
        <div
          style={{
            display: "grid",
            justifyContent: "center",
            alignContent: "start",
          }}
        >
          {props.recipe.steps.map((step, index) => (
            <code
              key={index}
              style={{
                margin: 4,
                backgroundColor: index === stepIndex ? "yellow" : undefined,
              }}
            >
              {step.key}
            </code>
          ))}
          <button onClick={() => setStepIndex(stepIndex - 1)}>
            previous step
          </button>
          <button onClick={() => setStepIndex(stepIndex + 1)}>next step</button>
        </div>
      </div>
    </div>
  );
};
