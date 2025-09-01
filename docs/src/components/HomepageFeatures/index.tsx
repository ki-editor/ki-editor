import clsx from "clsx";
import Heading from "@theme/Heading";
import { useColorMode } from '@docusaurus/theme-common';
import styles from "./styles.module.css";

type FeatureItem = {
  title: string;
  Svg: React.ComponentType<React.ComponentProps<"svg">>;
  description: JSX.Element;
};

const FeatureList: FeatureItem[] = [
  {
    title: "First-class syntax node interaction",
    Svg: require("@site/static/img/syntax-node.svg").default,
    description: (
      <>
        Bridge the gap between coding intent and action: manipulate syntax
        structures directly, avoiding mouse or keyboard gymnastics.
      </>
    ),
  },
  {
    title: "Multiple cursors",
    Svg: require("@site/static/img/multiple-cursors.svg").default,
    description: (
      <>
        Amplify your coding efficiency: wield multiple cursors for parallel
        syntax node operations, revolutionizing bulk edits and refactoring.
      </>
    ),
  },
  {
    title: "Redefine modal editing",
    Svg: require("@site/static/img/handdrawn-cube.svg").default,
    description: (
      <>
        Selection Modes standardize movements across words, lines, syntax nodes,
        and more, offering unprecedented flexibility and consistency.
      </>
    ),
  },
];

function Feature({ title, Svg, description }: FeatureItem) {
  const { colorMode } = useColorMode();

  return (
    <div className={clsx("col col--4")}>
      <div className="text--center">
        <Svg
          className={styles.featureSvg}
          role="img"
          style = {{
            fill: colorMode === 'dark' ? 'white' : 'black'
          }}
        />
      </div>
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): JSX.Element {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
