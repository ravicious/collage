port module Main exposing (main)

import Browser
import File exposing (File)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)
import Json.Decode as D


port sendImagesToJs : List D.Value -> Cmd msg


main =
    Browser.element
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        }



-- MODEL


type alias Model =
    List D.Value


init : () -> ( Model, Cmd a )
init _ =
    ( [], Cmd.none )



-- UPDATE


type Msg
    = GotFiles (List D.Value)


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotFiles files ->
            if List.length files >= 2 then
                ( files, sendImagesToJs <| List.take 2 files )

            else
                ( [], Cmd.none )


subscriptions _ =
    Sub.none



-- VIEW


view model =
    div []
        [ Html.form []
            [ input
                [ type_ "file"
                , accept "image/*"
                , multiple True
                , on "change" (D.map GotFiles filesDecoder)
                ]
                []
            ]
        , div [] [ text <| String.fromInt (List.length model) ++ " attachments" ]
        ]


filesDecoder : D.Decoder (List D.Value)
filesDecoder =
    D.at [ "target", "files" ] (D.list D.value)
