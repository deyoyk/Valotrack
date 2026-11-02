import { useMemo, useState } from "react";
import {
  Check,
  CircleHelp,
  Compass,
  Flag,
  Info,
  Map as MapIcon,
  Search,
  Trophy,
  X,
} from "lucide-react";
import calloutsData from "../data/callouts.json";
import { cn } from "../lib/format";

type Callout = {
  id: string;
  name: string;
  region: string;
  inGame: boolean;
  aliases: string[];
  description: string;
  x: number;
  y: number;
};

type MapData = {
  name: string;
  slug: string;
  art: string;
  callouts: Callout[];
  tips?: Record<string, string[]>;
};

const MAPS = calloutsData as unknown as Record<string, MapData>;
const MAP_NAMES = Object.keys(MAPS);

export default function Callouts() {
  const [mapName, setMapName] = useState(MAP_NAMES[0]);
  const [defender, setDefender] = useState(false);
  const [search, setSearch] = useState("");
  const [region, setRegion] = useState("All");
  const [selected, setSelected] = useState<Callout | null>(null);
  const [quiz, setQuiz] = useState(false);
  const [quizTarget, setQuizTarget] = useState<Callout | null>(null);
  const [guess, setGuess] = useState("");
  const [quizResult, setQuizResult] = useState<"correct" | "wrong" | null>(null);
  const [score, setScore] = useState({ correct: 0, total: 0 });

  const map = MAPS[mapName];
  const regions = useMemo(
    () => ["All", ...Array.from(new Set(map.callouts.map((item) => item.region)))],
    [map],
  );
  const rows = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return map.callouts.filter((item) => {
      if (region !== "All" && item.region !== region) return false;
      if (!needle) return true;
      return (
        item.name.toLowerCase().includes(needle) ||
        item.description.toLowerCase().includes(needle) ||
        item.aliases.some((alias) => alias.toLowerCase().includes(needle))
      );
    });
  }, [map, region, search]);

  const art = `/maps/${map.slug}-attack.svg`;
  const active = selected ?? rows[0] ?? map.callouts[0];
  const tips = map.tips?.[active?.region ?? ""] ?? [];

  function newQuestion(nextMap = map) {
    const list = nextMap.callouts.filter((callout) => callout.inGame || callout.region !== "Other");
    const pool = list.length ? list : nextMap.callouts;
    setQuizTarget(pool[Math.floor(Math.random() * pool.length)] ?? null);
    setGuess("");
    setQuizResult(null);
  }

  function startQuiz() {
    setQuiz(true);
    setSelected(null);
    setScore({ correct: 0, total: 0 });
    newQuestion();
  }

  function submitGuess() {
    if (!quizTarget || quizResult) return;
    const answer = guess.trim().toLowerCase();
    const valid = [quizTarget.name, ...quizTarget.aliases].some(
      (candidate) => candidate.trim().toLowerCase() === answer,
    );
    setQuizResult(valid ? "correct" : "wrong");
    setScore((current) => ({
      correct: current.correct + (valid ? 1 : 0),
      total: current.total + 1,
    }));
  }

  return (
    <div className="page callouts-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <MapIcon size={12} /> map callouts
          </span>
          <h1>Name every angle.</h1>
          <p>
            Official and community callouts pinned on the tactical map, with rotations,
            aliases and the tips that actually matter.
          </p>
        </div>
        <div className="head-status">
          <span className="checked">
            {map.callouts.length} callouts · patch 13.08
          </span>
        </div>
      </header>

      <div className="callout-toolbar">
        <div className="map-tabs">
          {MAP_NAMES.map((name) => (
            <button
              key={name}
              className={mapName === name ? "active" : ""}
              onClick={() => {
                setMapName(name);
                setSelected(null);
                setRegion("All");
                if (quiz) newQuestion(MAPS[name]);
              }}
            >
              {name}
            </button>
          ))}
        </div>
        <div className="toolbar-right">
          <button
            className={cn("button small", quiz && "active")}
            onClick={() => (quiz ? setQuiz(false) : startQuiz())}
          >
            <CircleHelp size={13} /> {quiz ? "Exit quiz" : "Quiz me"}
          </button>
          <div className="segmented">
            <button className={!defender ? "active" : ""} onClick={() => setDefender(false)}>
              Attacker view
            </button>
            <button className={defender ? "active" : ""} onClick={() => setDefender(true)}>
              Defender view
            </button>
          </div>
        </div>
      </div>

      <div className="callouts-grid">
        <section className="panel callout-map-panel">
          <div className="callout-map-head">
            <div>
              <span className="panel-label">{mapName}</span>
              <h2>Tactical map</h2>
            </div>
            <span className="callout-hint">
              <Compass size={12} /> click a pin
            </span>
          </div>
          <div className="callout-map">
            <div className={cn("callout-map-inner", defender && "flipped", quiz && "quiz-mode")}>
              <img src={art} alt={`${mapName} map`} draggable={false} />
              {map.callouts.map((callout) => (
                <button
                  key={callout.id}
                  className={cn(
                    "callout-pin",
                    active?.id === callout.id && !quiz && "active",
                    quiz && quizTarget?.id === callout.id && "quiz-target",
                    rows.some((row) => row.id === callout.id) ? "" : "dimmed",
                  )}
                  style={{ left: `${callout.x}%`, top: `${callout.y}%` }}
                  onClick={() => setSelected(callout)}
                  title={callout.name}
                >
                  <span className={defender ? "counter-rotate" : ""}>{callout.name}</span>
                </button>
              ))}
            </div>
          </div>
        </section>

        <section className={cn("panel callout-detail", quiz && "callout-quiz")}>
          {quiz ? (
            <>
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Callout quiz</span>
                  <h2>Name the pin</h2>
                </div>
                <span className="quiz-score">
                  <Trophy size={14} /> {score.correct}/{score.total}
                </span>
              </div>
              <p className="callout-description">
                The highlighted pin is hidden. Type its callout name or an accepted alias.
              </p>
              <form
                className="quiz-form"
                onSubmit={(event) => {
                  event.preventDefault();
                  submitGuess();
                }}
              >
                <input
                  value={guess}
                  onChange={(event) => setGuess(event.target.value)}
                  placeholder="Type the callout name"
                  disabled={quizResult !== null}
                  autoFocus
                />
                {quizResult === null ? (
                  <button className="button primary" type="submit" disabled={!guess.trim()}>
                    Check answer
                  </button>
                ) : (
                  <button className="button primary" type="button" onClick={() => newQuestion()}>
                    Next callout
                  </button>
                )}
              </form>
              {quizResult && (
                <div className={cn("quiz-result", quizResult)}>
                  {quizResult === "correct" ? <Check size={18} /> : <X size={18} />}
                  <div>
                    <strong>
                      {quizResult === "correct" ? "Correct" : "Not quite"}
                    </strong>
                    <span>
                      {quizTarget?.name}
                      {quizTarget?.region ? ` · ${quizTarget.region}` : ""}
                    </span>
                  </div>
                </div>
              )}
              <div className="quiz-help">
                <Info size={14} />
                <span>
                  The map keeps every other pin visible. Use the shape of the site, not the
                  labels, to find the answer.
                </span>
              </div>
            </>
          ) : (
            <>
              <div className="panel-heading">
                <div>
                  <span className="panel-label">{active?.region ?? "Callout"}</span>
                  <h2>{active?.name ?? "Map callout"}</h2>
                </div>
                {active?.inGame ? (
                  <span className="in-game-badge">in-game</span>
                ) : (
                  <span className="community-badge">community</span>
                )}
              </div>
              <p className="callout-description">{active?.description}</p>
              {active?.aliases.length ? (
                <div className="alias-row">
                  <span>Also called</span>
                  {active.aliases.map((alias) => (
                    <em key={alias}>{alias}</em>
                  ))}
                </div>
              ) : null}
              {tips.length ? (
                <div className="callout-tips">
                  <span className="panel-label">Playing it</span>
                  {tips.map((tip) => (
                    <div key={tip}>
                      <Info size={12} />
                      <span>{tip}</span>
                    </div>
                  ))}
                </div>
              ) : null}
            </>
          )}
        </section>
      </div>

      <div className={cn("callout-list-panel panel", quiz && "quiz-hidden")}>
        <div className="callout-list-toolbar">
          <div className="search-box">
            <Search size={14} />
            <input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search callouts"
            />
          </div>
          <div className="filter-tabs">
            {regions.map((item) => (
              <button
                key={item}
                className={region === item ? "active" : ""}
                onClick={() => setRegion(item)}
              >
                {item}
              </button>
            ))}
          </div>
        </div>
        <div className="callout-list">
          {rows.map((callout) => (
            <button
              key={callout.id}
              className={cn("callout-row", active?.id === callout.id && "active")}
              onClick={() => setSelected(callout)}
            >
              <span className="callout-row-icon">
                <Flag size={12} />
              </span>
              <span>
                <strong>{callout.name}</strong>
                <em>{callout.region}</em>
              </span>
              <span className="callout-row-desc">{callout.description}</span>
              <span className={callout.inGame ? "in-game-badge" : "community-badge"}>
                {callout.inGame ? "in-game" : "community"}
              </span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
