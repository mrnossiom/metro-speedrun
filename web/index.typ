#import "utils.typ": tabs

#show: document.with("index.html")

#show: html.html

#html.head[
  #html.meta(charset: "utf-8")
  #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")

  #html.link(rel: "stylesheet", href: "/styles.css")
  #html.title[Speedrun du Métro de Paris]
]

#show: html.body
#show: html.div

#title[Speedrun du Métro de Paris]

Ce site est la suite du leaderboard original sur le site de #link("https://www.mousti.lu/metro/")[mousti.lu].

= Classement

Le leaderboard original fait une distinction entre les temps Pré/Post-Pécresse.
Au vu du nombre de temps concernés (un seul), il nous semble plus justicieux de simplifier le leaderboard en combinant ces catégories.
La volonté de faire un nouveau classement est aussi de pouvoir fixer des règles claires sur les catégories (e.g. à quel moment démarrer le chronomètre);

= Tentatives

Le temps est décompté de quai à quai. La tentative commence quand le pied se détache du quai de départ et se termine quand on touche le quai d'arrivée.

#{
  let (category: categories, run: runs) = toml("runs.toml")

  let content = categories
    .pairs()
    .map(((key, category)) => {
      let runs = runs.filter(((run)) => run.category == category.name)

      let content = [
        #html.p(
          class: "category-description",
          category.description,
        )

        #if runs.len() == 0 [
          Pas encore de temps enregistrés dans cette catégorie...
        ] else {
          html.div(
            class: "table-wrapper",
            table(
              columns: 5,
              table.header([Place], [Temps], [Date], [Coureur·euse·s], [Commentaire]),
              ..runs
                .enumerate(start: 1)
                .map(((i, run)) => (
                  str(i),
                  run.time,
                  run.date,
                  run.runners.join[ & ],
                  run.attachments.map(src => link(src)[📹]).join() + " " + run.comment,
                ))
                .flatten(),
            ),
          )
        }
      ]
      (category.name, key, content)
    })

  tabs(content)
}

Pour soumettre une tentative, vous pouvez envoyer un mail à #html.span(class: "email").

= Ressources

// TODO: add a context note on all the links

- #link(
    "https://api.semanticscholar.org/CorpusID:114980283",
  )[Beckenbach, Isabel, et al. The S-Bahn Challenge in Berlin. 2015] #link(
    "https://ris.utwente.nl/ws/portalfiles/portal/6419213/FB-SBAHN-ChallengeV2.pdf",
  )[(alt)]

- #link("https://arxiv.org/pdf/1709.05948")[Sikora, Florian. The Shortest Way to Visit All Metro Lines in a City. 2018]

  #let wiki-linopt = "https://fr.wikipedia.org/wiki/Optimisation_lin%C3%A9aire_en_nombres_entiers";
  Ce papier modélise le problème de chemin le plus court pour de #link(wiki-linopt)[l'optimisation linéaire en nombre entiers] (_Integer Linear Programming_).

- #link("http://estacions.albertguillaumes.cat/")[Estacions i intercanviadors (Gares et correspondances)]

  Plans 3D des stations de métro

- #link(
    "https://web.archive.org/web/20070202173622/http://www.enseignement.polytechnique.fr/profs/informatique/Philippe.Chassignet/03-04/INF_431/dm_1.html",
  )[DM 1 : Visite touristique de Paris en Métro — Philippe Chassignet et Dominique Rossin (archive.org)]

  Devoir Maison de l'école Polytechnique en 2003

- #link("https://github.com/mrnossiom/metro-speedrun")[dépôt de code `metro-speedrun`]

  Contient le code de ce site ainsi que les algorithmes pour chercher les chemins les plus courts selon différents critères.

  Utilise les données de #link("https://www.wikidata.org/")[Wikidata] pour récupérer le réseau de transports d'une ville.
  Permet d'afficher le réseau sous forme de graphe *DOT* et d'afficher la solution.
  Implémente notament le modèle *ILP* de _Florian Sikora_ en *GLPK* et *SCIP-OPT* pour obtenir le chemin le plus court.

- #link("https://www.mousti.lu/metro/")[mousti.lu leaderboard]

  Les anciennes données de classements ont été récupérés sur ce site.

  Possède notament un guide pour le chemin optimal de _Florian Sikora_ #link("https://www.mousti.lu/metro/Tableau_Metro_Any.pdf")[(PDF)].
