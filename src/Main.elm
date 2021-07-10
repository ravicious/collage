port module Main exposing (main)

import Browser
import File exposing (File)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)
import Json.Decode as D


port sendImagesToJs : List D.Value -> Cmd msg


port imageProcessorStatus : (String -> msg) -> Sub msg


main =
    Browser.element
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        }



-- MODEL


type alias Model =
    Status


type Status
    = AwaitingInput
    | Processing
    | Done
    | Error ProcessingError


type ProcessingError
    = LessThanTwoImages


init : () -> ( Model, Cmd a )
init _ =
    ( AwaitingInput, Cmd.none )



-- UPDATE


type Msg
    = GotFiles (List D.Value)
    | ImageProcessorStatusUpdated


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotFiles files ->
            if List.length files >= 2 then
                ( Processing, sendImagesToJs <| List.take 2 files )

            else
                ( Error LessThanTwoImages, Cmd.none )

        ImageProcessorStatusUpdated ->
            ( Done, Cmd.none )


subscriptions _ =
    -- For now we only really handle the happy path, hence `always`.
    imageProcessorStatus (always ImageProcessorStatusUpdated)



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
        , div []
            [ case model of
                AwaitingInput ->
                    text ""

                Processing ->
                    text "Processing…"

                Done ->
                    text ""

                Error LessThanTwoImages ->
                    text "Can't make a collage with just one image"
            ]
        ]


filesDecoder : D.Decoder (List D.Value)
filesDecoder =
    D.at [ "target", "files" ] (D.list D.value)
