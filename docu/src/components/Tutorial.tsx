import { FitAddon } from "@xterm/addon-fit";
import { useEffect, useMemo, useState } from "react";
import { useXTerm, UseXTermProps } from "react-xtermjs";
import * as recipesData from "../../assets/recipes.json";
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
    <div style={{ display: "grid", gap: 64 }}>
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

const xtermOptions: UseXTermProps = {
  options: { fontSize: 20, cols: 60, rows: 10 },
};
export const Recipe = (props: { recipe: Recipe }) => {
  const { instance, ref } = useXTerm(xtermOptions);
  const fitAddon = new FitAddon();
  const [stepIndex, setStepIndex] = useState(0);
  useEffect(() => {
    // Load the fit addon
    instance?.loadAddon(fitAddon);

    const handleResize = () => fitAddon.fit();

    // Handle resize event
    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, [ref, instance]);
  useEffect(() => {
    const step = props.recipe.steps[stepIndex];
    instance?.write(step.term_output);
  }, [ref, instance, stepIndex]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <h2>{props.recipe.description}</h2>
      <div
        style={{
          display: "grid",
          justifyContent: "start",
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
          ‹
        </button>
        {props.recipe.steps.map((step, index) => (
          <button
            onClick={() => setStepIndex(index)}
            className={[
              "kbc-button",
              index === stepIndex ? "active" : undefined,
            ].join(" ")}
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
          ›
        </button>
      </div>
      <div
        ref={ref}
        style={{ justifySelf: "start", border: "1px solid black" }}
      />
    </div>
  );
};
