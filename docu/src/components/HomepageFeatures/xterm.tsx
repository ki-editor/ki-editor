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
      <link
        rel="stylesheet"
        href="https://unpkg.com/keyboard-css@1.2.4/dist/css/main.min.css"
      />

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

    instance?.write(step.term_output);

    // Handle resize event
    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, [ref, instance, stepIndex]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div>{props.recipe.description}</div>
      <div
        style={{
          display: "grid",
          justifyContent: "center",
          alignContent: "start",
          justifyItems: "center",
          gridAutoFlow: "column",
          gap: 8,
        }}
      >
        <button
          className="kbc-button"
          onClick={() => setStepIndex(Math.max(stepIndex - 1, 0))}
        >
          previous step
        </button>
        {props.recipe.steps.map((step, index) => (
          <button
            onClick={() => setStepIndex(index)}
            className={[
              "kbc-button",
              index === stepIndex ? "active" : undefined,
            ].join(" ")}
            style={{ margin: 4 }}
          >
            {step.key}
          </button>
        ))}
        <button
          className="kbc-button"
          onClick={() =>
            setStepIndex(Math.min(stepIndex + 1, props.recipe.steps.length - 1))
          }
        >
          next step
        </button>
      </div>
      <div ref={ref} style={{ height: "100%", width: "100%" }} />
    </div>
  );
};
