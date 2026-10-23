import Markdown from "react-markdown";

export function ReleaseNotes({ notes }: { notes: string }) {
  return (
    <div className="release-notes">
      <Markdown
        skipHtml
        disallowedElements={["img"]}
        components={{
          a: ({ children, href }) => (
            <span>
              {children}
              {href && <> ({href})</>}
            </span>
          ),
        }}
      >
        {notes || "No release notes supplied."}
      </Markdown>
    </div>
  );
}
