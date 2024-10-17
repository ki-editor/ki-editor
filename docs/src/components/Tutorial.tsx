import { useEffect, useMemo, useState } from "react";
import useBaseUrl from "@docusaurus/useBaseUrl";

import { useXTerm, UseXTermProps } from "react-xtermjs";
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
  terminal_height: z.number(),
  terminal_width: z.number(),
  similar_vim_combos: z.array(z.string()),
});

type Recipe = z.infer<typeof recipeSchema>;

export const Tutorial = (props: { filename: string }) => {
  const [recipes, setRecipes] = useState<Recipe[]>([]);
  const url = useBaseUrl(`/recipes/${props.filename}.json`);
  useEffect(() => {
    loadRecipes(url).then((recipes) => setRecipes(recipes ?? []));
  }, []);
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

async function loadRecipes(url: string) {
  const response = await fetch(url);
  const recipesData = await response.json();
  return z.array(recipeSchema).parse(recipesData.recipes_output);
}

const Recipe = (props: { recipe: Recipe }) => {
  const xtermOptions: UseXTermProps = useMemo(
    () => ({
      options: {
        fontSize: 20,
        cols: props.recipe.terminal_width,
        rows: props.recipe.terminal_height,
      },
    }),
    []
  );

  const { instance, ref } = useXTerm(xtermOptions);
  const [stepIndex, setStepIndex] = useState(0);
  useEffect(() => {
    const step = props.recipe.steps[stepIndex];
    instance?.write(step.term_output);
  }, [ref, instance, stepIndex]);

  return (
    <div
      style={{
        display: "grid",
        gap: 16,
        justifySelf: "start",
        overflow: "hidden",
      }}
    >
      <div
        style={{
          display: "grid",
          gridAutoFlow: "column",
          alignItems: "center",
        }}
      >
        <div style={{ display: "grid" }}>
          <h2>{props.recipe.description}</h2>
          <div
            style={{
              display: "grid",
              gridAutoFlow: "column",
              gap: 8,
              justifyContent: "start",
            }}
          >
            {props.recipe.similar_vim_combos.map((combo, index) => (
              <div
                key={index}
                style={{
                  display: "grid",
                  gridAutoFlow: "column",
                  gap: 4,
                  justifyContent: "start",
                  alignItems: "center",
                }}
              >
                <img
                  style={{ height: 24 }}
                  src={useBaseUrl("/img/vim-icon.svg")}
                />
                <code style={{ padding: "0 8px" }}>{combo}</code>
              </div>
            ))}
          </div>
        </div>
        <div
          style={{
            display: "grid",
            gap: 8,
            gridAutoFlow: "column",
            justifySelf: "end",
          }}
        >
          <button
            className="kbc-button"
            onClick={() => setStepIndex(Math.max(stepIndex - 1, 0))}
          >
            ‹
          </button>
          <button
            className="kbc-button"
            onClick={() =>
              setStepIndex(
                Math.min(stepIndex + 1, props.recipe.steps.length - 1)
              )
            }
          >
            ›
          </button>
        </div>
      </div>
      <div
        ref={ref}
        style={{ justifySelf: "start", border: "1px solid black" }}
      />
      <div
        style={{
          display: "grid",
          justifyContent: "start",
          alignContent: "start",
          justifyItems: "center",
          gap: 8,
        }}
      >
        <div
          style={{
            display: "grid",
            gap: 2,
            gridAutoFlow: "column",
            justifySelf: "start",
            overflowX: "auto",
            width: "100%",
          }}
        >
          {props.recipe.steps.map((step, index) => (
            <button
              onClick={() => setStepIndex(index)}
              className={[
                "kbc-button",
                index === stepIndex ? "active" : undefined,
              ].join(" ")}
              style={{ fontFamily: "monospace" }}
            >
              {step.key}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};
